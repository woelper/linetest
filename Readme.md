# linetest

A library to continuously measure, log and visualize throughput and latency potentially unstable network connections.

### Goals:
- Can be used to create long-running tests of network connections
- Cross platform library that aims to work on Linux/Mac/Win
- Provides a real-world speed test not tied to a specific provider/API
- Open Source

### How is data being gathered?

- Latency is currently evaluated by pinging `8.8.8.8`. This is configurable. Later this might be a list of hosts that has a candidate randomly picked or sourced from a mean value over multiple.

- Download speed is currently evaluated by downloading a series of ~20-50MB files from google and AWS in parallel. The total byte size is then divided by the actual time passed until all complete. While this is not the maximum your line could theoretically provide, it should give an indication about the real world throughput.

### Are there tools using this?
- There is an extremely simple command line utility provided in `linetest-cli/`.
- There is a gui application in development in `linetest-gui/`