use crate::svm::fun::Num::U24;
use bend::fun::Term;

pub enum SVMPrimitives {
    U24(u32),
}

impl SVMPrimitives {
    pub fn to_term(&self) -> Term {
        match self {
            SVMPrimitives::U24(inner) => bend::fun::Term::Num { val: U24(*inner) },
        }
    }

    pub fn from_term(term: Term) -> Self {
        match term {
            Term::Num { val: U24(inner) } => Self::U24(inner),
            _ => {
                term.display_pretty(0);
                todo!("unsupported term");
            }
        }
    }
}
