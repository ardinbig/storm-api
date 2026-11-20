# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.4](https://github.com/ardinbig/storm-api/releases/tag/v0.1.4) - 2026-08-05

### Added

- add idempotency middleware for protected routes
- extract auth middleware and openapi
- add house commission account seeding and protection mechanism
- enhance card creation error handling & add new tests for db errors
- update user roles to distinguish between super-admin and regular users
- update environment configuration and protect super-admin account from deletion
- implement pagination for consumptions and transactions endpoints
- link agents to stations with new station_id field and migration
- implement super-admin account seeding on application startup
- add DETELE endpoint for removing commission rates
- implement PATCH endpoint for updating agent details
- add transaction management with withdrawal and listing endpoints
- implement agent account management with CRUD operations, authentication, and customer registration
- add customer routes and handlers for CRUD operations and card-based lookup
- update database schema for cards, users, and consumption tables
- add routes and handlers for fuel consumption and pricing management
- add NFC card routes and handlers for CRUD operations and balance checks
- add commission and commission tier routes with handlers for listing & creating
- add category routes and handlers for listing, retrieving, and creating categories
- enhance application entry-point with structured logging and graceful shutdown
- add application router with health check and authentication middleware
- implement authentication routes and handlers for login, registration, and logout
- add authentication services with detailed documentation and password handling
- implement application state management and Redis caching utilities
- add PostgreSQL connection pool and initial database schema
- initialize the project with Docker support

### Other

- update README for idempotency support
- fix missing workflow permissions for CodeQL
- update configuration file
- add release automation with release-plz config
- enhance CI/CD configuration
- *(deps)* bump rand from 0.9.2 to 0.9.5
- add CodeQL configuration to exclude Rust test files
- update coverage generation step to install cargo-nextest alongside cargo-llvm-cov
- change checkout action to version 7 in CI configuration
- enhance CI configuration with improved steps
- bump version to 0.1.4 and update dependencies
- *(deps)* bump rand from 0.8.5 to 0.8.6
- Merge pull request #10 from ardinbig/develop
- add unit test for withdrawal handling missing house account
- remove locked flag from cargo commands in CI configuration
- update dependencies in CI configuration and increment API version
- add container names for database, redis, and api services
- refactor SQL queries for improved readability & performance
- *(deps)* bump rustls-webpki from 0.103.10 to 0.103.13
- update CI configuration to include permissions for content access
- update dependencies in Cargo.toml & remove unused rand crate
- rename card_id and remove unused balance check endpoint
- update README, CI config & Dockerfile with build dependencies
- update README with comprehensive API features and quickstart cmd
- enhance API endpoints with OpenAPI documentation
- add end-to-end tests and improve test app setup
- add authentication endpoint tests for registration, login, and logout functionality
- initial commit
