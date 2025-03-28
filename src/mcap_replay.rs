
/// Helper function to advance the mcap reader.

use std::borrow::Cow;
use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{anyhow, Context, Result};
use bytes::Buf;
use foxglove::{
    Channel, ChannelBuilder, PartialMetadata, Schema, 
    WebSocketServerBlockingHandle,
};

use mcap::records::{MessageHeader, Record, SchemaHeader};
use mcap::sans_io::read::{LinearReader, LinearReaderOptions, ReadAction};

pub fn advance_reader<R, F>(
    reader: &mut LinearReader,
    file: &mut R,
    mut handle_record: F,
) -> Result<bool>
where
    R: Read + Seek,
    F: FnMut(Record<'_>) -> Result<()>,
{
    if let Some(action) = reader.next_action() {
        match action? {
            ReadAction::NeedMore(count) => {
                let count = file.read(reader.insert(count))?;
                reader.set_written(count);
            }
            ReadAction::GetRecord { data, opcode } => {
                let record = mcap::parse_record(opcode, data)?;
                handle_record(record)?;
            }
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

#[derive(Default)]
pub struct Summary {
    path: PathBuf,
    schemas: HashMap<u16, Schema>,
    channels: HashMap<u16, Arc<Channel>>,
}

impl Summary {
    pub fn load_from_mcap(path: &Path) -> Result<Self> {
        let mut file = BufReader::new(File::open(path)?);

        // Read the last 28 bytes of the file to validate the trailing magic (8 bytes) and obtain
        // the summary start value, which is the first u64 in the footer record (20 bytes).
        let mut buf = Vec::with_capacity(28);
        file.seek(SeekFrom::End(-28)).context("seek footer")?;
        file.read_to_end(&mut buf).context("read footer")?;
        if !buf.ends_with(mcap::MAGIC) {
            return Err(anyhow!("bad footer magic"));
        }

        // Seek to summary section.
        let summary_start = buf.as_slice().get_u64_le();
        if summary_start == 0 {
            return Err(anyhow!("missing summary section"));
        }
        file.seek(SeekFrom::Start(summary_start))
            .context("seek summary")?;

        let mut reader = LinearReader::new_with_options(LinearReaderOptions {
            skip_start_magic: true,
            ..Default::default()
        });

        let mut summary = Summary {
            path: path.to_owned(),
            schemas: HashMap::new(),
            channels: HashMap::new(),
        };
        while advance_reader(&mut reader, &mut file, |rec| summary.handle_record(rec))
            .context("read summary")?
        {}

        Ok(summary)
    }

    /// Creates a new file stream.
    pub fn file_stream(&self) -> FileStream<'_> {
        FileStream::new(&self.path, &self.channels)
    }

    // Handles a record from the summary section.
    pub fn handle_record(&mut self, record: Record<'_>) -> Result<()> {
        match record {
            Record::Schema { header, data } => self.handle_schema(&header, data),
            Record::Channel(channel) => self.handle_channel(channel),
            _ => Ok(()),
        }
    }

    /// Caches schema information.
    pub fn handle_schema(
        &mut self,
        header: &SchemaHeader,
        data: Cow<'_, [u8]>,
    ) -> Result<(), anyhow::Error> {
        if header.id == 0 {
            return Err(anyhow!("invalid schema id"))?;
        }
        if let Entry::Vacant(entry) = self.schemas.entry(header.id) {
            let schema = Schema::new(&header.name, &header.encoding, data.into_owned());
            entry.insert(schema);
        }
        Ok(())
    }

    /// Registers a new channel.
    pub fn handle_channel(&mut self, record: mcap::records::Channel) -> Result<(), anyhow::Error> {
        if let Entry::Vacant(entry) = self.channels.entry(record.id) {
            let schema = self.schemas.get(&record.schema_id).cloned();
            let channel = ChannelBuilder::new(record.topic)
                .message_encoding(&record.message_encoding)
                .schema(schema)
                .build()?;
            entry.insert(channel);
        }
        Ok(())
    }
}

pub struct FileStream<'a> {
    pub path: PathBuf,
    channels: &'a HashMap<u16, Arc<Channel>>,
    time_tracker: Option<TimeTracker>,
}

impl<'a> FileStream<'a> {
    /// Creates a new file stream.
    pub fn new(path: &Path, channels: &'a HashMap<u16, Arc<Channel>>) -> Self {
        Self {
            path: path.to_owned(),
            channels,
            time_tracker: None,
        }
    }

    /// Streams the file content until `done` is set.
    pub fn stream_until(
        mut self,
        server: &WebSocketServerBlockingHandle,
        done: &Arc<AtomicBool>,
    ) -> Result<()> {
        let mut file = BufReader::new(File::open(&self.path)?);
        let mut reader = LinearReader::new();
        while !done.load(Ordering::Relaxed)
            && advance_reader(&mut reader, &mut file, |rec| {
                self.handle_record(server, rec);
                Ok(())
            })
            .context("read data")?
        {}
        Ok(())
    }

    /// Handles an mcap record parsed from the file.
    pub fn handle_record(&mut self, server: &WebSocketServerBlockingHandle, record: Record<'_>) {
        if let Record::Message { header, data } = record {
            self.handle_message(server, header, &data);
        }
    }

    /// Streams the message data to the server.
    pub fn handle_message(
        &mut self,
        server: &WebSocketServerBlockingHandle,
        header: MessageHeader,
        data: &[u8],
    ) {
        let tt = self
            .time_tracker
            .get_or_insert_with(|| TimeTracker::start(header.log_time));

        tt.sleep_until(header.log_time);

        if let Some(timestamp) = tt.notify() {
            server.broadcast_time(timestamp);
        }

        if let Some(channel) = self.channels.get(&header.channel_id) {
            channel.log_with_meta(
                data,
                PartialMetadata {
                    sequence: Some(header.sequence),
                    log_time: Some(header.log_time),
                    publish_time: Some(header.publish_time),
                },
            );
        }
    }
}

/// Helper for keep tracking of the relationship between a file timestamp and the wallclock.
pub struct TimeTracker {
    start: Instant,
    offset_ns: u64,
    now_ns: u64,
    notify_interval_ns: u64,
    notify_last: u64,
}
impl TimeTracker {
    /// Initializes a new time tracker, treating "now" as the specified offset from epoch.
    pub fn start(offset_ns: u64) -> Self {
        Self {
            start: Instant::now(),
            offset_ns,
            now_ns: offset_ns,
            notify_interval_ns: 1_000_000_000 / 60,
            notify_last: 0,
        }
    }

    /// Sleeps until the specified offset.
    pub fn sleep_until(&mut self, offset_ns: u64) {
        let abs = Duration::from_nanos(offset_ns.saturating_sub(self.offset_ns));
        let delta = abs.saturating_sub(self.start.elapsed());
        if delta >= Duration::from_micros(1) {
            std::thread::sleep(delta);
        }
        self.now_ns = offset_ns;
    }

    /// Periodically returns a timestamp reference to broadcast to clients.
    pub fn notify(&mut self) -> Option<u64> {
        if self.now_ns.saturating_sub(self.notify_last) >= self.notify_interval_ns {
            self.notify_last = self.now_ns;
            Some(self.now_ns)
        } else {
            None
        }
    }
}
