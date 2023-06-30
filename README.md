# Retake Connect

## Overview
Retake Connect is an open-source Extract, Transform, Load (ETL) library designed to connect SQL databases to embeddings databases in real-time. 
Existing "real time" change data capture (CDC) libraries like Debezium incur 10s+ latency per update and require complex setup; Connect is 10x faster and more performant.

## Table of Contents
- [Features](#features)
- [Installation](#installation)

## Features
- **Real-time Data Processing**: Monitor changes (inserts, updates, deletes) in your SQL databases and reflect them in your embeddings databases in <1s.
- **Scalable & Efficient**: Designed to handle large volumes of data and high-throughput workloads.
- **Wide Database Support**: Easily integrate with popular SQL databases like PostgreSQL, MySQL, etc., and embeddings databases like Pinecone, Chroma, etc.
- **Customizable Transformations**: Apply custom transformation functions to your data.
- **Simple Configuration**: Minimal setup required and easy to configure.
- **Well Documented**: Comprehensive documentation to get you started quickly.

## Installation
Connect can be installed using pip:

```sh
pip install retake-connect
```

## Contributing
We need to write a CONTRIBUTING.md

## License
We need to write a LICENSE.md

## Support
TBD