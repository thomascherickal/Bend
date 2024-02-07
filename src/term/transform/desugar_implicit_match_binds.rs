use crate::term::{Adt, Book, DefName, MatchNum, Pattern, Term, VarName};
use indexmap::IndexMap;

impl Book {
  pub fn desugar_implicit_match_binds(&mut self) {
    for def in self.defs.values_mut() {
      for rule in &mut def.rules {
        rule.body.desugar_implicit_match_binds(&self.ctrs, &self.adts);
      }
    }
  }
}

impl Term {
  pub fn desugar_implicit_match_binds(
    &mut self,
    ctrs: &IndexMap<DefName, DefName>,
    adts: &IndexMap<DefName, Adt>,
  ) {
    match self {
      Term::Match { scrutinee, .. } => {
        let scrutinee = if let Term::Var { nam } = scrutinee.as_ref() {
          nam.clone()
        } else {
          let Term::Match { scrutinee, arms } = std::mem::take(self) else { unreachable!() };

          let nam = VarName::new("%temp%scrutinee");

          *self = Term::Let {
            pat: Pattern::Var(Some(nam.clone())),
            val: scrutinee,
            nxt: Box::new(Term::Match { scrutinee: Box::new(Term::Var { nam: nam.clone() }), arms }),
          };

          nam
        };

        let (Term::Match { arms, .. } | Term::Let { nxt: box Term::Match { arms, .. }, .. }) = self else {
          unreachable!()
        };

        for (pat, body) in arms {
          match pat {
            Pattern::Var(_) => (),
            Pattern::Ctr(nam, pat_args) => {
              let adt = ctrs.get(nam).unwrap();
              let Adt { ctrs, .. } = adts.get(adt).unwrap();
              let ctr_args = ctrs.get(nam).unwrap();
              if pat_args.is_empty() && !ctr_args.is_empty() {
                // Implicit ctr args
                *pat_args =
                  ctr_args.iter().map(|x| Pattern::Var(Some(format!("{scrutinee}.{x}").into()))).collect();
              }
            }
            Pattern::Num(MatchNum::Zero) => (),
            Pattern::Num(MatchNum::Succ(Some(_))) => (),
            Pattern::Num(MatchNum::Succ(p @ None)) => {
              // Implicit num arg
              *p = Some(Some(format!("{scrutinee}-1").into()));
            }
            Pattern::Tup(_, _) => (),
            Pattern::List(..) => unreachable!(),
          }
          body.desugar_implicit_match_binds(ctrs, adts);
        }
      }
      Term::Let { pat: Pattern::Var(_), val: fst, nxt: snd }
      | Term::App { fun: fst, arg: snd, .. }
      | Term::Dup { val: fst, nxt: snd, .. }
      | Term::Tup { fst, snd }
      | Term::Sup { fst, snd, .. }
      | Term::Opx { fst, snd, .. } => {
        fst.desugar_implicit_match_binds(ctrs, adts);
        snd.desugar_implicit_match_binds(ctrs, adts);
      }
      Term::Lam { bod, .. } | Term::Chn { bod, .. } => {
        bod.desugar_implicit_match_binds(ctrs, adts);
      }
      Term::Era
      | Term::Ref { .. }
      | Term::Num { .. }
      | Term::Str { .. }
      | Term::Lnk { .. }
      | Term::Var { .. }
      | Term::Invalid => (),
      Term::Let { pat: _, .. } => {
        unreachable!("Expected destructor let expressions to have been desugared already")
      }
      Term::List { .. } => unreachable!("Should have been desugared already"),
    }
  }
}
