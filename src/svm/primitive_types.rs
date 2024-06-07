use bend::fun::Num::U24;
use bend::fun::Term;
use log::error;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum SVMPrimitives {
    U24(u32),
    Tup(Vec<SVMPrimitives>),
    Era,
}

impl SVMPrimitives {
    pub fn to_term(&self) -> Term {
        match self {
            SVMPrimitives::U24(inner) => bend::fun::Term::Num { val: U24(*inner) },
            SVMPrimitives::Tup(inner) => bend::fun::Term::Fan {
                fan: bend::fun::FanKind::Tup,
                tag: bend::fun::Tag::Static,
                els: inner.clone().iter().map(|e| e.clone().to_term()).collect(),
            },
            SVMPrimitives::Era => bend::fun::Term::Era,
        }
    }

    pub fn from_term(term: Term) -> Self {
        let term_c = term.clone();
        match term {
            Term::Num { val: U24(inner) } => Self::U24(inner),
            Term::Fan {
                fan: _,
                tag: _,
                ref els,
            } => {
                let els = els.clone();
                Self::Tup(
                    els.clone()
                        .iter()
                        .map(|e| SVMPrimitives::from_term(e.clone()))
                        .collect(),
                )
            }
            Term::Era {} => Self::Era,
            unsupported => {
                error!("unsupported term {:#?}", term_c.clone());
                unsupported.display_pretty(0);
                todo!("unsupported term");
            }
        }
    }
}
