# Retake Connect

## Overview
Retake is a real-time embeddings database with pre-built, real-time connectors to your "sources of truth" (i.e. Postgres, MySQL, etc.). This enables you to integrate embeddings with your existing databases effortlessly, ensuring that your embeddings are always up-to-date with the underlying data sources.

## Why Retake
- **Out-of-the-box data sync** Existing embeddings databases are data siloes that require complex and sometimes brittle mechanisms for data synchronization. Retake provides native connectors that allow seamless data synchronization without the need for extensive configuration or third-party tools.
- **True real-time updates** Most "real-time" data sync libraries are built on top of Debezium/Kafka, which introduce significant overhead latency on the order of 10s+ per transaction. Retake's connectors are optimized for low latency, ensuring that changes in your data sources are reflected in your embeddings in <1s.
- **Dead-simple Python SDK**  With just a few lines of Python code, you can set up data synchronization, query embeddings, and perform sophisticated data manipulations. 
- **Scalable & Efficient**: Designed to handle large volumes of data and high-throughput workloads.

## Getting Started
To get started with Retake, follow these simple steps:

```sh
pip install retake
```

## Contributing
We need to write a CONTRIBUTING.md

## License
We need to write a LICENSE.md

## Support
TBD