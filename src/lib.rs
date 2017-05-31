//! A hack to support deserialization of arbitrary trait objects.
//!
//! The core of the library rests upon the trio `TyConst`, `get_ty_const`, and
//! `get_ty_const_key`.  They provide a mechanism for looking up data
//! associated with a type using a persistent identifier ("key").
//!
//! `D: TyConst<T>` is trait used to define such data.  Each implementation
//! associates an arbitrary value of type `D` (i.e. `Self`) with the type
//! parameter `T`.  Conceptually, it's as if every `D` has its own table of
//! data, indexed by `T`.  Within a given table, every `T` is associated with
//! a unique `usize` key.
//!
//! `get_ty_const_key` returns the unique key for the data associated with `T`
//! in the table of `D`.  The key is persistent (serializable): it can be used
//! for a later execution of the same program.  The key is only guaranteed to
//! be unique for a given `D` (i.e. the key is meaningless without knowing
//! what `D` is).
//!
//! `get_ty_const` uses the key to retrieve the data `D` associated with `T`
//! without knowing what `T` was.  If the key is invalid, then `None` is
//! returned.
//!
//! ## Example
//!
//! For a more interesting example, see the [`serde`](serde/index.html)
//! submodule.
//!
//! ```
//! use detrojt::{TyConst, get_ty_const, get_ty_const_key};
//!
//! #[derive(Debug, PartialEq, Eq)]
//! struct Size(usize);
//!
//! impl<T: 'static> TyConst<T> for Size {
//!     fn get_data() -> Self { Size(std::mem::size_of::<T>()) }
//! }
//!
//! assert_eq!(get_ty_const(get_ty_const_key::<Size, ()>()), Some(Size(0)));
//! assert_eq!(get_ty_const(get_ty_const_key::<Size, i64>()), Some(Size(8)));
//! assert_eq!(get_ty_const::<Size>(1), None);
//! ```

extern crate serde as libserde;
extern crate serde_json;

pub mod serde;

use std::any::TypeId;
use std::io::Write;
use std::marker::PhantomData;

#[cfg(any(unix))]
unsafe fn ptr_try_read<T>(p: *const T) -> Option<T> {
    let mut f = std::fs::File::create("/dev/random").unwrap();
    let s = std::slice::from_raw_parts(p as *const u8, std::mem::size_of::<T>());
    if f.write(s).is_ok() {
        Some(std::ptr::read(p))
    } else {
        None
    }
}

#[derive(Debug)]
#[repr(C)]
struct TraitObject {
    data: *mut (),
    vtable: *mut (),
}

#[derive(Debug)]
#[repr(C)]
struct Vtable {
    destructor: fn(*mut ()),
    size: usize,
    align: usize,
}

trait TyConstImpl<D> {
    fn get_type_id(&self) -> TypeId;
    fn get(&self) -> D;
}

const MAGIC: usize = 0x625f405b5af9;

struct Dummy<T: ?Sized> {
    _dummy: [u8; MAGIC],
    phantom: PhantomData<T>,
}

impl<D: TyConst<T>, T: ?Sized + 'static> TyConstImpl<D> for Dummy<T> {
    fn get_type_id(&self) -> TypeId {
        TypeId::of::<D>()
    }
    fn get(&self) -> D {
        D::get_data()
    }
}

/// This represents a mapping from a type T to some data of type `Self` (also
/// referred to as `D` in other places).
///
/// (The ordering of type parameters here is needed to avoid problems due to
/// orphan rules.)
pub trait TyConst<T: ?Sized + 'static>: Sized + 'static {
    /// Retrieve the data.
    fn get_data() -> Self;
}

/// Get the key associated with `TyConst<T>` for `D`.  Uses of this function
/// determine what goes into the type constant table for `D`.
pub fn get_ty_const_key<D: TyConst<T>, T: ?Sized + 'static>() -> usize {
    unsafe {
        let r0: TraitObject = std::mem::transmute(&() as &Send);
        let p: &'static Dummy<T> = std::mem::transmute(&());
        let r: TraitObject = std::mem::transmute(p as &TyConstImpl<D>);
        (r.vtable as usize) - (r0.vtable as usize)
    }
}

/// Get the data in the impl for the type that matches the given key.
/// If no such impl is found, returns `None`.
///
/// **Due to limitations of the current implementation, calling this on an
/// invalid key may sometimes cause arbitrary code execution (or a crash if
/// you're lucky).**
pub fn get_ty_const<D: 'static>(key: usize) -> Option<D> {
    unsafe {
        let r0: TraitObject = std::mem::transmute(&() as &Send);
        let r = TraitObject {
            data: &mut (),
            vtable: ((r0.vtable as usize) + key) as _,
        };
        match ptr_try_read(r.vtable as *const Vtable) {
            None => return None,
            Some(ref vt) if vt.size == MAGIC && vt.align == 1 => (),
            _ => return None,
        }
        let r: &TyConstImpl<D> = std::mem::transmute(r);
        if r.get_type_id() != TypeId::of::<D>() {
            return None;
        }
        Some(r.get())
    }
}
