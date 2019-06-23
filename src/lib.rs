//! # Bumblebee
//!
//! Bumblebee is a JSON transformer with simple built in rules that can easily be implemented by even
//! the average user. It is designed to be extensible, simple to use and serializable for easy
//! storage and creation within service and apps.
//!
//! Source values that are not found or are incompatible will show up as `null` values in
//! the output.
//!
//! ```rust
//! use bumblebee::prelude::*;
//! use bumblebee::errors::Result;
//!
//! fn test_example() -> Result<()> {
//!       let trans = TransformerBuilder::default()
//!         .add_direct("user_id", "id")?
//!         .add_direct("full-name", "name")?
//!         .add_flatten(
//!                "nicknames",
//!                "",
//!                FlattenOps {
//!                    recursive: true,
//!                    prefix: Some("nickname"),
//!                    separator: Some("_"),
//!                    manipulation: None,
//!                },
//!            )?
//!         .add_direct("nested.inner.key", "prev_nested")?
//!         .add_direct("nested.my_arr[1]", "prev_arr")?
//!         .build()?;
//!     let input = r#"
//!         {
//!             "user_id":"111",
//!             "full-name":"Dean Karn",
//!             "nicknames":["Deano","Joey Bloggs"],
//!             "nested": {
//!                 "inner":{
//!                     "key":"value"
//!                 },
//!                 "my_arr":[null,"arr_value",null]
//!             }
//!         }"#;
//!     let expected = r#"{"id":"111","name":"Dean Karn","nickname_1":"Deano","nickname_2":"Joey Bloggs","prev_arr":"arr_value","prev_nested":"value"}"#;
//!     let res = trans.apply_from_str(input)?;
//!     assert_eq!(expected, serde_json::to_string(&res)?);
//!     Ok(())
//! }
//! ```
//!
//! or direct from struct to struct
//!
//! ```rust
//! use bumblebee::prelude::*;
//! use bumblebee::errors::Result;
//! use serde::{Serialize, Deserialize};
//!
//! fn test_struct() -> Result<()> {
//!     #[derive(Debug, Serialize)]
//!     struct From {
//!         existing: String,
//!     }
//!
//!     #[derive(Debug, Deserialize, PartialEq)]
//!     struct To {
//!         new: String,
//!     }
//!
//!     let trans = TransformerBuilder::default()
//!         .add_direct("existing", "new")?
//!         .build()?;
//!
//!     let from = From {
//!         existing: String::from("existing_value"),
//!     };
//!
//!     let expected = To {
//!         new: String::from("existing_value"),
//!     };
//!     let res: To = trans.apply_to(from)?;
//!     assert_eq!(expected, res);
//!     Ok(())
//! }
//! ```
//!
pub mod errors;
pub mod namespace;
pub mod rules;
pub mod transformer;
mod tree;

pub mod prelude {
    pub use crate::rules::FlattenOps;
    pub use crate::transformer::TransformerBuilder;
}
