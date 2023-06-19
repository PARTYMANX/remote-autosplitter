# Remote Autosplitter for Livesplit
Connects to a livesplit server and runs an Auto Split Runtime splitter, useful for two PC streaming/capture setups.

Ever needed an autosplitter to run on a separate PC, but didn't want to set up your splits, OBS, or possibly a remote plugin (while also wanting game time to work properly)?  This will do that for you.  This is a small client made to connect to a livesplit server and run an autosplitter, forwarding any commands to the server.

Instructions: start your Livesplit server, then start Remote Autosplitter with the following arguments:

```
remote-autosplitter.exe <path to autosplitter wasm file> <address:port>
```

This is extremely barebones at the moment and only really supports basic operations and has no real UI.  

Stuff that's missing:
* Friendly UI (Rust currently has very little infrastructure for native UI, sadly)
* Autosplitter settings
* Server timer state (From what I can tell, Livesplit is currently missing functionality for this to work.  so as it stands, there's no way to tell if the timer is running or not in reality)
* Lots of hardening (if the server disappears, I don't really know what happens.  I can only assume it will panic.  Same thing when the autosplitter panics)