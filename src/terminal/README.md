# Goals
- Creates conpty process
- Pipes input through parser
- Manage all terminal buffers
    - Scrollback buffer
    - Alternate buffer
- Terminal features (independent of window implementation)
    - Cursor position
    - Emit events to window if required (title change, resize) via trait
    - Receive events from window via trait

# Data structures
- Scrollback buffer structure
    - Should be easy to read from render thread for rendering
    - Shoud have scrollback efficiently
    - Should be resizable
    - Should be space efficient
    - Should be fast to write to from conpty output thread

