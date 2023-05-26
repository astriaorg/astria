# Astria

Astria replaces centralized sequencers, allowing many rollups to share a single decentralized network of sequencers that’s simple and permissionless to join. This shared sequencer network provides out-of-the-box censorship resistance, fast block confirmations, and atomic cross-rollup composability – all while retaining each rollup’s sovereignty.

This repository contains the custom Astria components that make up the Astria network. Other components of the Astria network can be found in the [astriaorg](https://github.com/astriaorg) organization. 

To run locally, we utilize a dev-cluster which can be found at [astriaorg/dev-cluster](https://github.com/astriaorg/dev-cluster). 

To learn more about Astria, please visit [astria.org](https://astria.org).

## Components

* [conductor](https://github.com/astriaorg/astria/tree/main/crates/astria-conductor)
* [gossipnet](https://github.com/astriaorg/astria/tree/main/crates/astria-gossipnet)
* [proto](https://github.com/astriaorg/astria/tree/main/crates/astria-proto)
* [rs-cnc](https://github.com/astriaorg/astria/tree/main/crates/astria-rs-cnc)
* [sequencer-relayer](https://github.com/astriaorg/astria/tree/main/crates/astria-sequencer-relayer)

## Contributing

Pull requests should be created against the `main` branch. In general, we follow the "fork-and-pull" Git workflow.

1. Fork the repo on GitHub
2. Clone the project to your own machine
3. Commit changes to your own branch
4. Push your work back up to your fork
5. Submit a Pull request so that we can review your changes

NOTE: Be sure to merge the latest from upstream before making a pull request!

## Issues

If you encounter any issues while using this project or have any questions, please open an issue in this repository [here](https://github.com/astriaorg/astria/issues).
