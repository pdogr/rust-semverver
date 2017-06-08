use rustc::hir::def::Export;
use rustc::session::Session;

use std::collections::BTreeSet;
use std::cmp::Ordering;

use syntax::symbol::Ident;

use syntax_pos::Span;

/// The categories we use when analyzing changes between crate versions.
///
/// These directly correspond to the semantic versioning spec, with the exception that
/// some breaking changes are categorized as "technically breaking" - that is, [1]
/// defines them as non-breaking when introduced to the standard libraries.
///
/// [1]: https://github.com/rust-lang/rfcs/blob/master/text/1105-api-evolution.md
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChangeCategory {
    /// Patch change - no change to the public API of a crate.
    Patch,
    /// A backwards-compatible change.
    NonBreaking,
    /// A breaking change that is very unlikely to cause breakage.
    TechnicallyBreaking,
    /// A breaking, backwards-incompatible change.
    Breaking,
}

pub use self::ChangeCategory::*;

impl<'a> Default for ChangeCategory {
    fn default() -> ChangeCategory {
        Patch
    }
}

impl<'a> From<&'a UnaryChange> for ChangeCategory {
    fn from(change: &UnaryChange) -> ChangeCategory {
        match *change {
            UnaryChange::Addition(_) => TechnicallyBreaking,
            UnaryChange::Removal(_) => Breaking,
        }
    }
}

impl<'a> From<&'a BinaryChangeType> for ChangeCategory {
    fn from(type_: &BinaryChangeType) -> ChangeCategory {
        match *type_ {
            Unknown => Breaking,
        }
    }
}

impl<'a> From<&'a BinaryChange> for ChangeCategory {
    fn from(change: &BinaryChange) -> ChangeCategory {
        From::from(change.type_())
    }
}

impl<'a> From<&'a Change> for ChangeCategory {
    fn from(change: &Change) -> ChangeCategory {
        match *change {
            Change::Unary(ref u) => From::from(u),
            Change::Binary(ref b) => From::from(b),
        }
    }
}

/// The types of changes we identify.
#[derive(Clone, Debug)]
pub enum BinaryChangeType {
    /// An unknown change is any change we don't yet explicitly handle.
    Unknown,
}

pub use self::BinaryChangeType::*;

/// A change record.
///
/// Consists of all information we need to compute semantic versioning properties of
/// the change, as well as data we use to output it in a nice fashion.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
pub enum Change {
    Unary(UnaryChange),
    Binary(BinaryChange),
}

impl Change {
    pub fn new_addition(export: Export) -> Change {
        Change::Unary(UnaryChange::Addition(export))
    }

    pub fn new_removal(export: Export) -> Change {
        Change::Unary(UnaryChange::Removal(export))
    }

    pub fn new_binary(type_: BinaryChangeType, old: Export, new: Export) -> Change {
        Change::Binary(BinaryChange::new(type_, old, new))
    }
}

/// A change record of a change that introduced or removed an item.
///
/// It is important to note that the `Eq` and `Ord` instances are constucted to only
/// regard the span and path of the associated item export. This allows us to sort them
/// by appearance in the source, but possibly could create conflict later on.
pub enum UnaryChange {
    /// An item has been added.
    Addition(Export),
    /// An item has been removed.
    Removal(Export),
}

impl UnaryChange {
    fn export(&self) -> &Export {
        match *self {
            UnaryChange::Addition(ref e) | UnaryChange::Removal(ref e) => e,
        }
    }

    pub fn span(&self) -> &Span {
        &self.export().span
    }

    pub fn ident(&self) -> &Ident {
        &self.export().ident
    }

    pub fn type_(&self) -> &'static str {
        match *self {
            UnaryChange::Addition(_) => "Addition",
            UnaryChange::Removal(_) => "Removal",
        }
    }
}

impl PartialEq for UnaryChange {
    fn eq(&self, other: &UnaryChange) -> bool {
        self.span() == other.span()
    }
}

impl Eq for UnaryChange {}

impl PartialOrd for UnaryChange {
    fn partial_cmp(&self, other: &UnaryChange) -> Option<Ordering> {
        self.span().partial_cmp(other.span())
    }
}

impl Ord for UnaryChange {
    fn cmp(&self, other: &UnaryChange) -> Ordering {
        self.span().cmp(other.span())
    }
}

pub struct BinaryChange {
    type_: BinaryChangeType,
    old: Export,
    new: Export,
}

impl BinaryChange {
    pub fn new(type_: BinaryChangeType, old: Export, new: Export) -> BinaryChange {
        BinaryChange {
            type_: type_,
            old: old,
            new: new,
        }
    }

    pub fn type_(&self) -> &BinaryChangeType {
        &self.type_
    }

    pub fn new_span(&self) -> &Span {
        &self.new.span
    }

    pub fn old_span(&self) -> &Span {
        &self.old.span
    }

    pub fn ident(&self) -> &Ident {
        &self.old.ident
    }
}

impl PartialEq for BinaryChange {
    fn eq(&self, other: &BinaryChange) -> bool {
        self.new_span() == other.new_span()
    }
}

impl Eq for BinaryChange {}

impl PartialOrd for BinaryChange {
    fn partial_cmp(&self, other: &BinaryChange) -> Option<Ordering> {
        self.new_span().partial_cmp(other.new_span())
    }
}

impl Ord for BinaryChange {
    fn cmp(&self, other: &BinaryChange) -> Ordering {
        self.new_span().cmp(other.new_span())
    }
}

/// The total set of changes recorded for two crate versions.
#[derive(Default)]
pub struct ChangeSet {
    /// The currently recorded changes.
    changes: BTreeSet<Change>,
    /// The most severe change category already recorded.
    max: ChangeCategory,
}

impl ChangeSet {
    /// Add a change to the set and record it's category for later use.
    pub fn add_change(&mut self, change: Change) {
        let cat: ChangeCategory = From::from(&change);

        if cat > self.max {
            self.max = cat;
        }

        self.changes.insert(change);
    }

    /// Format the contents of a change set for user output.
    ///
    /// TODO: replace this with something more sophisticated.
    pub fn output(&self, session: &Session) {
        println!("max: {:?}", self.max);

        for change in &self.changes {
            match *change {
                Change::Unary(ref c) => {
                    println!("  {}: {}", c.type_(), c.ident().name.as_str());
                },
                Change::Binary(ref c) => {
                    println!("  {:?}: {}", c.type_(), c.ident().name.as_str());
                },
            }
            // session.span_warn(*change.span(), "change");
            // span_note!(session, change.span(), "S0001");
        }
    }
}

#[cfg(test)]
pub mod tests {
    use quickcheck::*;
    pub use super::*;

    use rustc::hir::def::Def;

    use std::cmp::{max, min};

    use syntax_pos::BytePos;
    use syntax_pos::hygiene::SyntaxContext;
    use syntax_pos::symbol::{Ident, Interner};

    #[derive(Clone, Debug)]
    pub struct Span_(Span);

    impl Span_ {
        pub fn inner(self) -> Span {
            self.0
        }
    }

    impl Arbitrary for Span_ {
        fn arbitrary<G: Gen>(g: &mut G) -> Span_ {
            let a: u32 = Arbitrary::arbitrary(g);
            let b: u32 = Arbitrary::arbitrary(g);
            Span_(Span {
                      lo: BytePos(min(a, b)),
                      hi: BytePos(max(a, b)),
                      ctxt: SyntaxContext::empty(),
                  })
        }
    }

    #[derive(Clone, Debug)]
    pub enum UnaryChangeType {
        Removal,
        Addition,
    }

    impl Arbitrary for UnaryChangeType {
        fn arbitrary<G: Gen>(g: &mut G) -> UnaryChangeType {
            g.choose(&[UnaryChangeType::Removal, UnaryChangeType::Addition]).unwrap().clone()
        }
    }

    impl<'a> From<&'a UnaryChangeType> for ChangeCategory {
        fn from(change: &UnaryChangeType) -> ChangeCategory {
            match *change {
                UnaryChangeType::Addition => TechnicallyBreaking,
                UnaryChangeType::Removal => Breaking,
            }
        }
    }

    pub type UnaryChange_ = (UnaryChangeType, Span_);

    /// We build these by hand, because symbols can't be sent between threads.
    fn build_unary_change(t: UnaryChangeType, s: Span) -> UnaryChange {
        let mut interner = Interner::new();
        let ident = Ident {
            name: interner.intern("test"),
            ctxt: SyntaxContext::empty(),
        };

        let export = Export {
            ident: ident,
            def: Def::Err,
            span: s,
        };

        match t {
            UnaryChangeType::Addition => UnaryChange::Addition(export),
            UnaryChangeType::Removal => UnaryChange::Removal(export),
        }
    }

    quickcheck! {
        /// The `Ord` instance of `Change` is transitive.
        fn ord_change_transitive(c1: UnaryChange_, c2: UnaryChange_, c3: UnaryChange_) -> bool {
            let ch1 = build_unary_change(c1.0, c1.1.inner());
            let ch2 = build_unary_change(c2.0, c2.1.inner());
            let ch3 = build_unary_change(c3.0, c3.1.inner());

            let mut res = true;

            if ch1 < ch2 && ch2 < ch3 {
                res &= ch1 < ch3;
            }

            if ch1 == ch2 && ch2 == ch3 {
                res &= ch1 == ch3;
            }

            if ch1 > ch2 && ch2 > ch3 {
                res &= ch1 > ch3;
            }

            res
        }

        /// The maximal change category for a change set gets computed correctly.
        fn max_change(changes: Vec<UnaryChange_>) -> bool {
            let mut set = ChangeSet::default();

            let max = changes.iter().map(|c| From::from(&c.0)).max().unwrap_or(Patch);

            for &(ref change, ref span) in changes.iter() {
                set.add_change(
                    Change::Unary(build_unary_change(change.clone(), span.clone().inner())));
            }

            set.max == max
        }

        /// Difference in spans implies difference in `Change`s.
        fn change_span_neq(c1: UnaryChange_, c2: UnaryChange_) -> bool {
            let s1 = c1.1.inner();
            let s2 = c2.1.inner();

            if s1 != s2 {
                let ch1 = build_unary_change(c1.0, s1);
                let ch2 = build_unary_change(c2.0, s2);

                ch1 != ch2
            } else {
                true
            }
        }
    }
}