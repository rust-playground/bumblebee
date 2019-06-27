# Bumblebee &emsp; [![Build Status]][travis] [![Latest Version]][crates.io]

[Build Status]: https://api.travis-ci.org/rust-playground/bumblebee.svg?branch=master
[travis]: https://travis-ci.org/rust-playground/bumblebee
[Latest Version]: https://img.shields.io/crates/v/bumblebee.svg
[crates.io]: https://crates.io/crates/bumblebee

**Bumblebee is a JSON transformer with simple built in rules that can easily be implemented by even the average user. It is designed to be extensible, simple to use and serializable for easy storage and creation within service and apps.**

---

```toml
[dependencies]
bumblebee = "0.1"
```

## Example usages
```rust
use bumblebee::prelude::*;
use bumblebee::errors::Result;
fn test_example() -> Result<()> {
      let trans = TransformerBuilder::default()
        .add_direct("user_id", "id")?
        .add_direct("full-name", "name")?
        .add_flatten(
               "nicknames",
               "",
               FlattenOps {
                   recursive: true,
                   prefix: Some("nickname"),
                   separator: Some("_"),
                   manipulation: None,
               },
           )?
        .add_direct("nested.inner.key", "prev_nested")?
        .add_direct("nested.my_arr[1]", "prev_arr")?
        .build()?;
    let input = r#"
        {
            "user_id":"111",
            "full-name":"Dean Karn",
            "nicknames":["Deano","Joey Bloggs"],
            "nested": {
                "inner":{
                    "key":"value"
                },
                "my_arr":[null,"arr_value",null]
            }
        }"#;
    let expected = r#"{"id":"111","name":"Dean Karn","nickname_1":"Deano","nickname_2":"Joey Bloggs","prev_arr":"arr_value","prev_nested":"value"}"#;
    let res = trans.apply_from_str(input)?;
    assert_eq!(expected, serde_json::to_string(&res)?);
    Ok(())
}
```

or when you want to do struct to struct transformations

```rust
use bumblebee::prelude::*;
use bumblebee::errors::Result;
use serde::{Serialize, Deserialize};
fn test_struct() -> Result<()> {
    #[derive(Debug, Serialize)]
    struct From {
        existing: String,
    }
    #[derive(Debug, Deserialize, PartialEq)]
    struct To {
        new: String,
    }
    let trans = TransformerBuilder::default()
        .add_direct("existing", "new")?
        .build()?;
    let from = From {
        existing: String::from("existing_value"),
    };
    let expected = To {
        new: String::from("existing_value"),
    };
    let res: To = trans.apply_to(from)?;
    assert_eq!(expected, res);
    Ok(())
}
```

#### License

<sup>
Licensed under either of <a href="LICENSE-APACHE">Apache License, Version
2.0</a> or <a href="LICENSE-MIT">MIT license</a> at your option.
</sup>

<br>

<sub>
Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in Bumblebee by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
</sub>
