# Rust Documentation MCP Server Technical Specification

## Abstract

This document specifies the requirements for a Model Context Protocol (MCP) server that provides access to Rust crate documentation, version information, and project metadata.

## Status of This Document

This document specifies requirements for the Rust Documentation MCP Server and provides information to the community.

## Conventions and Terminology

The key words "MUST", "MUST NOT", "REQUIRED", "SHALL", "SHALL NOT", "SHOULD", "SHOULD NOT", "RECOMMENDED", "NOT RECOMMENDED", "MAY", and "OPTIONAL" in this document are to be interpreted as described in BCP 14 [RFC2119] [RFC8174] when, and only when, they appear in all capitals.

## Introduction

The Rust Documentation MCP Server enables AI assistants to access Rust crate documentation and metadata through a standardized protocol. This specification defines the requirements for implementing such a server.

## Server Requirements

### General

The server MUST implement the Model Context Protocol (MCP) specification.

The server MUST provide access to documentation through the public-api crate's analysis of rustdoc JSON output.

The server MUST support caching of documentation and crate information.

The server SHOULD implement cache invalidation based on time-to-live (TTL) values.

The server MUST handle concurrent requests safely.

### Resources

The server MUST implement the following resources:

#### Crate Documentation Resource

The server MUST provide a resource at path `crate/{name}/docs`.

The server MUST accept an optional version parameter in the query string.

The server MUST generate rustdoc JSON using the nightly toolchain.

The server MUST parse the rustdoc JSON using the public-api crate.

The server MUST return documentation in a structured format containing public API items, modules, types, and traits.

The server SHOULD cache documentation results to improve performance.

#### Crate Information Resource

The server MUST provide a resource at path `crate/{name}/info`.

The server MUST execute the `cargo info` command to retrieve crate information.

The server MUST parse and return the latest version of the crate.

The server MUST return all available versions of the crate.

The server MUST return all available features and their descriptions.

The server MUST return the crate's dependencies.

The server SHOULD cache crate information with a shorter TTL than documentation.

#### Project Metadata Resource

The server MUST provide a resource at path `project/metadata`.

The server MUST execute the `cargo metadata` command with format version 1.

The server MUST return workspace member information.

The server MUST return dependency information.

The server MUST return target information.

The server MUST return feature information.

The server SHOULD NOT cache project metadata as it should reflect the current state.

### Caching Requirements

The server MUST implement a caching system for documentation and crate information.

The server MUST store cache entries with timestamps.

The server MUST implement time-to-live (TTL) for cache entries.

The server MUST validate cache entries before returning them.

The server MUST remove invalid cache entries when detected.

The server SHOULD implement a maximum cache size limit.

The server SHOULD implement a least-recently-used (LRU) eviction policy when the cache is full.

### Error Handling

The server MUST return appropriate error responses for all failure cases.

The server MUST return a "not found" error when a requested crate does not exist.

The server MUST return an "invalid parameters" error when an invalid version is specified.

The server MUST return an "internal error" for command execution failures.

The server MUST return an "internal error" for parsing failures.

The server MUST provide error messages that are helpful for debugging.

The server SHOULD log detailed error information for debugging purposes.

### Configuration

The server MUST support configuration of cache TTL values.

The server MUST support configuration of maximum cache size.

The server SHOULD support configuration of the nightly toolchain path.

The server MAY support configuration of custom cargo command paths.

## Security Considerations

The server MUST validate all input parameters to prevent command injection.

The server MUST handle file paths securely to prevent path traversal attacks.

The server SHOULD implement rate limiting for resource-intensive operations.

The server SHOULD implement memory limits for cache storage.

## Performance Considerations

The server SHOULD optimize cache hit rates for commonly accessed crates.

The server SHOULD implement concurrent request handling efficiently.

The server SHOULD minimize the number of cargo command executions.

The server MAY implement background cache warming for popular crates.

## Implementation Notes

Implementers SHOULD refer to the examples document for guidance on implementing these requirements.

Implementers MUST ensure thread safety when implementing caching mechanisms.

Implementers SHOULD use appropriate Rust async runtime features for command execution.

## References

### Normative References

- [RFC2119] Key words for use in RFCs
- [RFC8174] Ambiguity of Uppercase vs Lowercase
- Model Context Protocol Specification
- Rust public-api crate documentation
- Cargo command documentation

### Informative References

- Rust Documentation RFC
- rustdoc JSON output format
- Cargo metadata format
