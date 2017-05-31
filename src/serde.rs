//! A collection of helper traits that can be used to add serialization
//! support to user-defined traits.
//!
//! See [`Trait`](trait.Trait.html) for more information.
//!
//! ## Example
//!
//! ```
//! extern crate detrojt;
//! extern crate serde;
//! extern crate serde_json;
//!
//! use detrojt::serde::{HasInterDeserialize, Trait, deserialize, serialize};
//!
//! // a minimal example trait that supports Debug
//! trait MyTrait: Trait<serde_json::Value, MyTraitObj> + std::fmt::Debug {}
//!
//! // add a single implementation
//! impl MyTrait for String {}
//!
//! // Create a wrapper type for the trait object.
//! // The wrapper type must support From<T: MyTrait> and HasInterDeserialize.
//! #[derive(Debug)]
//! struct MyTraitObj(Box<MyTrait>);
//!
//! impl<T: MyTrait + 'static> From<T> for MyTraitObj {
//!     fn from(t: T) -> Self {
//!         MyTraitObj(Box::new(t))
//!     }
//! }
//!
//! impl HasInterDeserialize for MyTraitObj {
//!     type InterDeserialize = serde_json::Value;
//! }
//!
//! impl serde::Serialize for MyTraitObj {
//!     fn serialize<S>(&self, s: S) -> Result<S::Ok, S::Error>
//!         where S: serde::Serializer
//!     {
//!         serialize(&*self.0, s)
//!     }
//! }
//!
//! impl<'de> serde::Deserialize<'de> for MyTraitObj {
//!     fn deserialize<D>(d: D) -> Result<Self, D::Error>
//!         where D: serde::Deserializer<'de>
//!     {
//!         deserialize(d)
//!     }
//! }
//!
//! fn main() {
//!     let a: MyTraitObj = MyTraitObj(Box::new("hello world".to_owned()));
//!     let s = serde_json::to_value(&a).unwrap();
//!     let b: MyTraitObj = serde_json::from_value(s).unwrap();
//!     assert_eq!(format!("{:?}", a), format!("{:?}", b));
//! }
//! ```

use std::fmt;
use std::error::Error;
use std::marker::PhantomData;
use serde_json;
use libserde as serde;
use self::serde::{Serialize, Serializer, Deserializer};
use self::serde::de::DeserializeOwned;
use self::serde::ser::SerializeTuple;
use super::{TyConst, get_ty_const, get_ty_const_key};

/// Intermediate format used to serialize the trait object.
///
/// An implementation is provided for `serde_json::Value`, but it's easy to
/// implement this for other kinds of serialization formats.
pub trait InterSerialize: Serialize + Sized + 'static {
    fn inter_serialize<T: Serialize>(t: &T) -> Result<Self, Box<Error>>;
}

impl InterSerialize for serde_json::Value {
    fn inter_serialize<T: Serialize>(t: &T) -> Result<Self, Box<Error>> {
        serde_json::to_value(t).map_err(Into::into)
    }
}

/// Intermediate format used to deserialize the trait object.
///
/// An implementation is provided for `serde_json::Value`, but it's easy to
/// implement this for other kinds of serialization formats.
pub trait InterDeserialize: DeserializeOwned + 'static {
    fn inter_deserialize<T>(self) -> Result<T, Box<Error>>
        where T: DeserializeOwned;
}

impl InterDeserialize for serde_json::Value {
    fn inter_deserialize<T>(self) -> Result<T, Box<Error>>
        where T: DeserializeOwned
    {
        serde_json::from_value(self).map_err(Into::into)
    }
}

/// Used to associate the trait object with an intermediate deserializer.
pub trait HasInterDeserialize: 'static {
    type InterDeserialize: InterDeserialize;
}

/// Supertrait for adding serialization support to user-defined traits.
///
/// If you want to define your own trait with serialization support, you will
/// want to inherit from this trait and specify:
///
///   - `S`: the intermediate serialization format,
///   - `U`: the trait object type, which also indirectly specifies the
///     intermediate deserialization format through `HasInterDeserialize`.
///
/// The intermediate serialization formats are used to work around the fact
/// that trait objects have to be specialized for a concrete format.  For the
/// most part you can just pick a generic format such as `serde_json::Value`.
///
/// The trait object type `U` should be the boxed form of your user-defined
/// trait.  You probably have to wrap it in a newtype to avoid circular trait
/// declarations.
///
/// In order for serialization to work, concrete implementations of your trait
/// must satisfy the bound `Into<U> + Serialize + DeserializeOwned + 'static`.
pub trait Trait<S: InterSerialize, U: HasInterDeserialize> {
    /// Serialize the inner object.
    ///
    /// You should not implement this trait.  It is automatically provided
    /// through the blanket implementation.
    fn serialize_inner(&self) -> Result<S, Box<Error>>;

    /// Retrieve the key associated with the deserializer
    ///
    /// You should not implement this trait.  It is automatically provided
    /// through the blanket implementation.
    fn ty_const_key(&self) -> usize;
}

impl<S, U, T> Trait<S, U> for T
    where S: InterSerialize,
          U: HasInterDeserialize,
          T: Into<U> + Serialize + DeserializeOwned + 'static
{
    fn serialize_inner(&self) -> Result<S, Box<Error>> {
        S::inter_serialize(self)
    }
    fn ty_const_key(&self) -> usize {
        get_ty_const_key::<TraitObjDeserializer<U>, Self>()
    }
}

/// Serialize the given trait object.  You can plug this method directly into
/// your trait object's `Serialize` implementation.
pub fn serialize<S, U, T, S2>(boxed: &T, s: S2) -> Result<S2::Ok, S2::Error>
    where S: InterSerialize,
          U: HasInterDeserialize,
          T: Trait<S, U> + ?Sized,
          S2: Serializer
{
    let mut s = s.serialize_tuple(2)?;
    s.serialize_element(&boxed.ty_const_key())?;
    s.serialize_element(&boxed.serialize_inner()
                        .map_err(serde::ser::Error::custom)?)?;
    s.end()
}

struct TraitObjDeserializer<U>(fn(U::InterDeserialize) -> Result<U, Box<Error>>)
    where U: HasInterDeserialize;

trait DeserializeInner<U: HasInterDeserialize> {
    fn deserialize_inner(d: U::InterDeserialize) -> Result<U, Box<Error>>;
}

impl<U, T> DeserializeInner<U> for T
    where U: HasInterDeserialize,
          T: Into<U> + DeserializeOwned + ?Sized + 'static
{
    fn deserialize_inner(d: U::InterDeserialize) -> Result<U, Box<Error>> {
        d.inter_deserialize::<T>().map(Into::into)
    }
}

impl<U, T> TyConst<T> for TraitObjDeserializer<U>
    where U: HasInterDeserialize,
          T: Into<U> + DeserializeOwned + ?Sized + 'static
{
    fn get_data() -> Self {
        TraitObjDeserializer(T::deserialize_inner)
    }
}

struct Visitor<U>(PhantomData<U>)
    where U: HasInterDeserialize;

impl<'de, U> serde::de::Visitor<'de> for Visitor<U>
    where U: HasInterDeserialize,
{
    type Value = U;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a seq")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
        A: serde::de::SeqAccess<'de>
    {
        let k = seq.next_element()?
        .ok_or(serde::de::Error::missing_field("missing key"))?;
        let TraitObjDeserializer(tod) = get_ty_const(k)
            .ok_or(serde::de::Error::invalid_value(
                serde::de::Unexpected::Unsigned(k as u64), &self))?;
        let d = seq.next_element()?
        .ok_or(serde::de::Error::missing_field("missing inner object"))?;
        tod(d).map_err(serde::de::Error::custom)
    }
}

/// Deserialize the given trait object.  You can plug this method directly
/// into your trait object's `Deserialize` implementation.
pub fn deserialize<'de, U, D>(d: D) -> Result<U, D::Error>
    where D: Deserializer<'de>,
          U: HasInterDeserialize
{
    d.deserialize_tuple(2, Visitor(PhantomData))
}
