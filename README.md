### Foxglove Controllable Camera SDK Demo - UnregisterFoxgloveCam

This uses the Foxglove SDK to stream a file to Foxglove over websocket, and also provide a controllable set of camera topics that allow the user to control the perspective of an image panel in the Foxglove App.

The goal of this is to provide a camera with a (potentially) greater degree of freedom to the user and allow them to record their camera movements as messages to an mcap file. 


## How to use:
1. clone repo
2. `cargo run -- --file <path>` (see CLI options for more)
3. Open connection to websocket: `ws://localhost:8765`
4. click back into the terminal where `cargo run` was called and you can use the `keys` to control the camera. 

CLI Options:
  - `--file <path>` path to the file that you want to stream to foxglove
  - `--loop` add if you want to loop the file after it's finished
  - `--write` whether you want to write everything sent back to an MCAP file (including the set of controlled camera topics)

The camera is controlled by typing into the terminal where the server was started, the keys are as follows:
 - W -> move forward
 - A -> look left
 - S -> move backward
 - D -> look right
 - Q -> roll cam counter-clockwise
 - E -> roll cam clockwise
 - `<Spacebar>` -> stop movement
 - Ctrl-C -> quit

## How this was accomplished:

I don't know rust, so I regrettably relied a decent amount on Cursor to fix my issues. I did do some manual refactoring and adjustments and putting things together. Most of the camera maths was generated :/.
I did learn a bunch in the process and definitely familiarized myself with the language a bit more. 

A lot of this was cannibalized from the quickstart and the mcap-replay.


## Didn't get to:
 - steering doesn't become relative to the roll of the camera
 - Wanted to create a service that could receive a frame_id from Foxglove to put the sdk-camera frame on.
 - Create an extension panel to capture keys inputs within the app to publish to the server to control the camera
 - adding yaw
