# memcomparable

[![Crate](https://img.shields.io/crates/v/memcomparable.svg)](https://crates.io/crates/memcomparable)
[![Docs](https://docs.rs/memcomparable/badge.svg)](https://docs.rs/memcomparable)
[![CI](https://github.com/risingwavelabs/memcomparable/workflows/CI/badge.svg?branch=main)](https://github.com/risingwavelabs/memcomparable/actions)

A memcomparable serialization format.

The memcomparable format allows comparison of two values by using the simple memcmp function.

## Installation

Add the `memcomparable` to your Cargo.toml:

```sh
$ cargo add memcomparable
```

## Usage

```rust
// serialize
let key1 = memcomparable::to_vec(&"hello").unwrap();
let key2 = memcomparable::to_vec(&"world").unwrap();
assert!(key1 < key2);

// deserialize
let v1: String = memcomparable::from_slice(&key1).unwrap();
let v2: String = memcomparable::from_slice(&key2).unwrap();
assert_eq!(v1, "hello");
assert_eq!(v2, "world");
```

### Optional Features

- `decimal`: Enable (de)serialization for Decimal type.

See [the documentation](https://docs.rs/memcomparable) for more details.

## License

Apache License 2.0. Please refer to [LICENSE](LICENSE) for more information.
