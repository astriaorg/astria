fn main() {}


mod grandparent {
    pub mod parent {
        // Should not trigger on this reexport
        pub(super) use self::child::Trait as OtherTrait;
    
        mod child {
            pub trait Trait {
                fn method(&self);
            }
        }
    }
}

mod grandparent_2 {
    pub(super) mod parent {
        // Should not trigger on this reexport either
        pub use self::child::Trait as OtherTrait;
    
        mod child {
            pub trait Trait {
                fn method(&self);
            }
        }
    }
}