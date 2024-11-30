# Async-Pub

A generic async publisher (ie. logger) implementation.

Simply create a type implementing Publisher that processes updates in sequence. This Publisher can be stateful (ie. batch messages and send as a block) and will be cleaned up with drop provider .close() is called on the logger.

The logger type is intended to be static initialized so as to be globally available and shared across the program.

The publisher runs on a background thread so logger.send() calls return almost immediately.

## TODO
- Avoid taking exclusive lock coving all counters when adding a new counter
  - Use a tree with locks at each node?
- Connection to remote
- Remote receiver
- Counter drop / publish on shutdown
- Ligher weight string repr: for communication? for local calls?
- Multi increment counters
- Perf: don't shift on every increment
