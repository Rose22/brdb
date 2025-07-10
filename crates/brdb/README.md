# BRDB-RS

This library provides an interface for reading and writing [Brickadia](https://brickadia.com/)'s World files, which are stored in the `.brdb` format.

It also contains code for assisting with parsing msgpack-schema files as defined in [Zeblote's Brickadia msgpac-schema Gist](https://gist.github.com/Zeblote/053d54cc820df3bccad57df676202895). Some undocumented changes to this format are required to fully read/write `.brdb` files.

The `.brz` format is described in [Zeblote's Brickadia brz Gist](https://gist.github.com/Zeblote/0fc682b9df1a3e82942b613ab70d8a04).

## API

See [Examples](./examples/) to see how to read/write worlds.

TODO...

## Notes

- Webassembly support requires [rusqlite](https://github.com/rusqlite/rusqlite/pull/1643) to support the `wasm32-unknown-unknown` target. When this is merged, the `brdb` crate should be able to support WebAssembly.
- This library does not contain every in-game asset name (item classes, etc) so a world with those values needs to be parsed to determine their respective values.
- The structs and component data inside worlds may change as Brickadia updates. The game should support migrating old worlds, but newly created worlds may have unexpected fields in them.

## Liability

Use these libraries on your saves at your own risk:

- This library may generate invalid `.brdb` files, which may cause the game (or your computer) to crash or behave unexpectedly. **Report these bugs to the Brickadia team.**
- This library may modify the contents of your `.brdb` files in ways that are not easily recoverable. **Make backups of worlds you plan to modify with this library.**