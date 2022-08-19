# mio-edge-triggered

This repo provides a minimal breaking example for edge triggered behavior
on Windows. Mio aims to support edge-triggered behavior, which means it "only delivers
events when changes occur on the monitored file descriptor" (from
[epoll](https://linux.die.net/man/7/epoll) docs describing `EPOLLET`).

This program does the following:

1. Initialize a TcpListener and connect a TcpStream. The listener writes
back anything that it receives from the stream
2. Registers for read + write events on the stream
3. Writes to the stream + waits until it is readable
4. Reads from the stream
5. Writes to the stream + waits until it is readable

When run with `cargo run`, this program will exit cleanly on macos + linux
but will panic on windows.

On Windows, in step 5 we never receive a readable notification. This is
because on Windows the selector clears the source's interest [flags on event receipt](
https://github.com/tokio-rs/mio/blob/master/src/sys/windows/selector.rs#L230-L233)
(e.g. this occurs in step 3 here).  The flags are reset by re-registering
which occurs only after a non-blocking i/o
operation (see
[here](https://github.com/tokio-rs/mio/blob/master/src/sys/windows/mod.rs#L82-L97)).

The discrepancy can be "fixed" by adding a non-blocking read after 4, as is
done in most (all?) of the
[tests](https://github.com/tokio-rs/mio/blob/master/tests/tcp_stream.rs#L107) in `mio` where this difference might
surface .

To be fully consistent with the `EPOLLET` from `epoll`, step 4 of
completing a successful read operation should reset (at least) the
readable interest on the source.
