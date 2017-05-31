/// ```
/// use detrojt::{TyConst, get_ty_const, get_ty_const_key};
///
/// #[derive(Debug, PartialEq, Eq)]
/// struct Size(usize);
///
/// impl<T: 'static> TyConst<T> for Size {
///     fn get_data() -> Self { Size(std::mem::size_of::<T>()) }
/// }
///
/// assert_eq!(get_ty_const(get_ty_const_key::<Size, ()>()), Some(Size(0)));
/// assert_eq!(get_ty_const(get_ty_const_key::<Size, i64>()), Some(Size(8)));
/// assert_eq!(get_ty_const::<Size>(1), None);
/// ```
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

/// This represents a mapping from a type T to some data of type `D`.
///
/// (The ordering of type parameters here is needed to avoid problems due to
/// orphan rules.)
pub trait TyConst<T: ?Sized + 'static>: Sized + 'static {
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
/// **Although unlikely, calling this on an invalid key may cause arbitrary
/// code execution (or a crash if you're lucky).**
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

#[cfg(test)]
mod tests {
    use super::*;
    use std;

    #[derive(Debug, PartialEq, Eq)]
    struct Size(usize);

    impl<T: 'static> TyConst<T> for Size {
        fn get_data() -> Self { Size(std::mem::size_of::<T>()) }
    }

    #[test]
    fn it_works() {
        assert_eq!(get_ty_const(get_ty_const_key::<Size, ()>()), Some(Size(0)));
        assert_eq!(get_ty_const(get_ty_const_key::<Size, i64>()), Some(Size(8)));
        assert_eq!(get_ty_const::<Size>(1), None);
    }
}
