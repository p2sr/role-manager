// Derived from https://stackoverflow.com/a/45795699/5960285

use std::borrow::Borrow;
use std::collections::HashMap;
use std::hash::{Hash, Hasher};
use serde::Deserialize;

trait KeyPair<A, B> {
    /// Obtains the first element of the pair.
    fn a(&self) -> &A;
    /// Obtains the second element of the pair.
    fn b(&self) -> &B;
}

impl<'a, A, B> Borrow<dyn KeyPair<A, B> + 'a> for (A, B)
    where
        A: Eq + Hash + 'a,
        B: Eq + Hash + 'a,
{
    fn borrow(&self) -> &(dyn KeyPair<A, B> + 'a) {
        self
    }
}

impl<A: Hash, B: Hash> Hash for dyn KeyPair<A, B> + '_ {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.a().hash(state);
        self.b().hash(state);
    }
}

impl<A: Eq, B: Eq> PartialEq for dyn KeyPair<A, B> + '_ {
    fn eq(&self, other: &Self) -> bool {
        self.a() == other.a() && self.b() == other.b()
    }
}

impl<A: Eq, B: Eq> Eq for dyn KeyPair<A, B> + '_ {}

#[derive(Deserialize, Debug)]
pub struct MultiKeyHashMap<A: Eq + Hash, B: Eq + Hash, C> {
    map: HashMap<(A, B), C>,
}

impl<A: Eq + Hash, B: Eq + Hash, C> MultiKeyHashMap<A, B, C> {
    pub fn get(&self, a: &A, b: &B) -> Option<&C> {
        self.map.get(&(a, b) as &dyn KeyPair<A, B>)
    }

    pub fn insert(&mut self, a: A, b: B, v: C) {
        self.map.insert((a, b), v);
    }
}

impl<A: Eq + Hash, B: Eq + Hash, C> Default for MultiKeyHashMap<A, B, C> {
    fn default() -> Self {
        MultiKeyHashMap {
            map: HashMap::new()
        }
    }
}

impl<A, B> KeyPair<A, B> for (A, B) {
    fn a(&self) -> &A {
        &self.0
    }
    fn b(&self) -> &B {
        &self.1
    }
}
impl<A, B> KeyPair<A, B> for (&A, &B) {
    fn a(&self) -> &A {
        self.0
    }
    fn b(&self) -> &B {
        self.1
    }
}
