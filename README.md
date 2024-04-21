# Async-Pub

A generic async publisher (ie. logger) implementation.

Simply create a type implementing Publisher that processes updates in sequence. This Publisher can be stateful (ie. batch messages and send as a block) and will be cleaned up with drop provider .close() is called on the logger.

The logger type is intended to be static initialized so as to be globally available and shared across the program.

The publisher runs on a background thread so logger.send() calls return almost immediately.