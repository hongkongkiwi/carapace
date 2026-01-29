// Console shim for QuickJS - provides console.log that calls into Rust
function log() {
    var args = Array.prototype.slice.call(arguments);
    var message = args.map(String).join(' ');
    __carapace_console_log(message);
}
