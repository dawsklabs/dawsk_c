mod def;

use std::marker::PhantomData;
use std::ops::{Index, IndexMut};

use self::def::{BodyArena, DefTables, ExprArena, ScopeArena};

pub trait Idx {
    fn new(idx: usize) -> Self;
    fn index(self) -> usize;
}

pub struct IndexVec<I: Idx, T> {
    items: Vec<T>,
    _marker: PhantomData<I>,
}

impl<I: Idx, T> IndexVec<I, T> {
    pub fn new() -> Self {
        Self {
            items: Vec::new(),
            _marker: PhantomData,
        }
    }

    pub fn push(&mut self, value: T) -> I {
        let id = I::new(self.items.len());
        self.items.push(value);
        id
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = (I, &T)> {
        self.items.iter().enumerate().map(|(i, v)| (I::new(i), v))
    }
}

impl<I: Idx, T> Index<I> for IndexVec<I, T> {
    type Output = T;

    fn index(&self, index: I) -> &Self::Output {
        &self.items[index.index()]
    }
}

impl<I: Idx, T> IndexMut<I> for IndexVec<I, T> {
    fn index_mut(&mut self, index: I) -> &mut Self::Output {
        &mut self.items[index.index()]
    }
}

pub struct HIRCtx {
    pub defs: DefTables,
    pub bodies: BodyArena,
    pub exprs: ExprArena,
    pub scopes: ScopeArena,
}
