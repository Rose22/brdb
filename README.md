# BRDB

This repo provides interfaces for reading and writing [Brickadia](https://brickadia.com/)'s World files, which are stored in the `.brdb` format.

It also contains code for assisting with parsing msgpack-schema files as defined in [Zeblote's Brickadia msgpac-schema Gist](https://gist.github.com/Zeblote/053d54cc820df3bccad57df676202895). Some undocumented changes to this format are required to fully read/write `.brdb` files.

## Implementations

- Rust [brdb](./crates/brdb)
- JS/TS: TODO

## Format

- TODO: describe file tree structure
- TODO: describe revision mechanism and dates
- TODO: describe blobs, compression, and hashing
- TODO: describe msgpack-schema format
- TODO: describe SoA (Structure of Arrays) format
- TODO: describe flat arrays and limitations
- TODO: describe shared schemas
- TODO: describe entity/component `.mps` files and the data that comes after the SoAs

## Liability

Use these libraries on your saves at your own risk:

- This library may generate invalid `.brdb` files, which may cause the game (or your computer) to crash or behave unexpectedly. **Report these bugs to the Brickadia team.**
- This library may modify the contents of your `.brdb` files in ways that are not easily recoverable. **Make backups of worlds you plan to modify with this library.**