# Blossom Server in Rust

Simple implementation of a blossom server. I'm learning to use rust, any feedback is welcome, no matter how harsh.

_This software is in alpha, don't use in production._

### TODO
- Performance
  - [ ] large files, in the gigabyte range
  - [ ] millions of blobs stored
  - [ ] streaming response for large files
- Tracing
  - [x] turn off
  - [x] output to stdout
  - [x] open telemetry to otlp exporter with uptrace
- configuration
  - [ ] able to specify max upload size
  - [ ] able to specify min upload size
  - [ ] able to specify allowed mime types
