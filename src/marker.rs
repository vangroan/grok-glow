use std::{cell::Cell, marker::PhantomData};

pub type Invariant = PhantomData<Cell<()>>;
