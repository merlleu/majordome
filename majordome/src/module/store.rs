use std::{
    any::{Any, TypeId},
    collections::HashMap,
    hash::BuildHasherDefault,
};

use super::AppModPointer;

#[derive(Default)]
pub struct TypeIdHasherDoNotUse {
    value: u64,
}

impl std::hash::Hasher for TypeIdHasherDoNotUse {
    #[inline]
    fn finish(&self) -> u64 {
        self.value
    }

    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        debug_assert_eq!(bytes.len(), 8);
        self.value = u64::from_ne_bytes(bytes.try_into().unwrap());
    }
}

#[derive(Default)]
pub struct AnyMapByKey {
    pub raw: HashMap<TypeId, Box<dyn Any + Send + Sync>, BuildHasherDefault<TypeIdHasherDoNotUse>>,
}

impl AnyMapByKey {
    #[inline]
    pub fn get<T>(&self) -> Option<&T::Target>
    where
        T: AppModPointer + 'static,
    {
        self.raw
            .get(&TypeId::of::<T>())
            .map(|any| any.downcast_ref_unchecked_::<T::Target>())
    }

    pub fn insert<T>(&mut self, value: T::Target) -> Option<Box<T::Target>>
    where
        T: AppModPointer + 'static,
    {
        self.raw
            .insert(
                TypeId::of::<T>(),
                Box::new(value) as Box<dyn Any + Send + Sync>,
            )
            .and_then(|any| any.downcast::<T::Target>().ok())
    }

    pub fn contains<T>(&self) -> bool
    where
        T: AppModPointer + 'static,
    {
        self.raw.contains_key(&TypeId::of::<T>())
    }

    pub fn len(&self) -> usize {
        self.raw.len()
    }
}

#[derive(Default)]
pub struct AnyMap {
    pub raw: HashMap<TypeId, Box<dyn Any + Send + Sync>, BuildHasherDefault<TypeIdHasherDoNotUse>>,
}

impl AnyMap {
    #[inline]
    pub fn get<T>(&self) -> Option<&T>
    where
        T: 'static + Send + Sync,
    {
        self.raw
            .get(&TypeId::of::<T>())
            .map(|any| any.downcast_ref_unchecked_::<T>())
    }

    pub fn insert<T>(&mut self, value: T) -> Option<Box<T>>
    where
        T: 'static + Send + Sync,
    {
        self.raw
            .insert(
                TypeId::of::<T>(),
                Box::new(value) as Box<dyn Any + Send + Sync>,
            )
            .and_then(|any| any.downcast::<T>().ok())
    }

    pub fn remove<T>(&mut self) -> Option<Box<T>>
    where
        T: 'static + Send + Sync,
    {
        self.raw
            .remove(&TypeId::of::<T>())
            .and_then(|any| any.downcast::<T>().ok())
    }
}

trait FastDowncast {
    fn downcast_ref_unchecked_<T: Any>(&self) -> &T;
}

impl FastDowncast for dyn Any + Send + Sync {
    #[inline]
    fn downcast_ref_unchecked_<T: Any>(&self) -> &T {
        debug_assert!(self.is::<T>());
        unsafe { &*(self as *const dyn Any as *const T) }
    }
}

#[cfg(test)]
mod tests {

    #[derive(Debug, PartialEq)]
    struct A {
        a: i32,
    }

    #[derive(Debug, PartialEq)]
    struct B {
        b: i32,
    }

    #[derive(Debug, PartialEq)]
    struct C {
        c: i32,
    }

    // #[async_trait::async_trait]
    // impl super::AppModPointer for A {
    //     type Target = Self;

    // }

    // #[async_trait::async_trait]
    // impl super::AppModPointer for B {
    //     type Target = Self;
    // }

    // impl super::AppModPointer for C {
    //     type Target = Self;
    // }

    #[test]
    fn test_all() {
        // let mut map = super::AnyMapByKey::new();

        // map.insert::<A>(A { a: 1 });
        // map.insert::<B>(B { b: 2 });
        // map.insert::<C>(C { c: 3 });

        // assert_eq!(map.get::<A>(), Some(&A { a: 1 }));
        // assert_eq!(map.get::<B>(), Some(&B { b: 2 }));
        // assert_eq!(map.get::<C>(), Some(&C { c: 3 }));

        // map.remove::<A>();
        // assert_eq!(map.get::<A>(), None);

        // map.insert::<B>(B { b: 4 });
        // assert_eq!(map.get::<B>(), Some(&B { b: 4 }));

        // map.clear();

        // assert_eq!(map.get::<A>(), None);

        let mut map2 = super::AnyMap::default();

        map2.insert(A { a: 1 });
        map2.insert(B { b: 2 });
        map2.insert(C { c: 3 });

        assert_eq!(map2.get::<A>(), Some(&A { a: 1 }));
        assert_eq!(map2.get::<B>(), Some(&B { b: 2 }));
        assert_eq!(map2.get::<C>(), Some(&C { c: 3 }));
    }
}
