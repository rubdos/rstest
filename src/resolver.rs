/// Define `Resolver` trait and implement it on some hashmaps and also define the `Resolver` tuple
/// composition. Provide also some utility functions related to how to create a `Resolver` and
/// resolving render.
///
use std::borrow::Cow;
use std::collections::HashMap;

use proc_macro2::{Ident, Span};
use syn::{parse_quote, Expr};

use crate::parse::Fixture;

pub(crate) mod fixtures {
    use super::*;

    pub(crate) fn get<'a>(fixtures: impl Iterator<Item = &'a Fixture>) -> impl Resolver + 'a {
        fixtures
            .map(|f| (f.name.to_string(), extract_resolve_expression(f).into()))
            .collect::<HashMap<_, Expr>>()
    }

    fn extract_resolve_expression(fixture: &Fixture) -> syn::Expr {
        let name = &fixture.name;
        let positional = &fixture.positional;
        let pname = format!("partial_{}", positional.len());
        let partial = Ident::new(&pname, Span::call_site());
        parse_quote! { #name::#partial(#(#positional), *) }
    }

    #[cfg(test)]
    mod should {
        use super::*;
        use crate::test::{assert_eq, *};

        #[test]
        fn resolve_by_use_the_given_name() {
            let data = vec![fixture("pippo", vec![])];
            let resolver = get(data.iter());

            let resolved = resolver.resolve(&ident("pippo")).unwrap().into_owned();

            assert_eq!(resolved, "pippo::partial_0()".ast());
        }
    }
}

/// A trait that `resolve` the given ident to expression code to assign the value.
pub(crate) trait Resolver {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>>;
}

impl<'a> Resolver for HashMap<String, &'a Expr> {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>> {
        let ident = ident.to_string();
        self.get(&ident).map(|&c| Cow::Borrowed(c))
    }
}

impl<'a> Resolver for HashMap<String, Expr> {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>> {
        let ident = ident.to_string();
        self.get(&ident).map(|c| Cow::Borrowed(c))
    }
}

impl<R1: Resolver, R2: Resolver> Resolver for (R1, R2) {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>> {
        self.0.resolve(ident).or_else(|| self.1.resolve(ident))
    }
}

impl<R: Resolver + ?Sized> Resolver for &R {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>> {
        (*self).resolve(ident)
    }
}

impl<R: Resolver + ?Sized> Resolver for Box<R> {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>> {
        (**self).resolve(ident)
    }
}

impl Resolver for (String, Expr) {
    fn resolve(&self, ident: &Ident) -> Option<Cow<Expr>> {
        if self.0 == ident.to_string() {
            Some(Cow::Borrowed(&self.1))
        } else {
            None
        }
    }
}

#[cfg(test)]
mod should {
    use super::*;
    use crate::test::{assert_eq, *};
    use syn::parse_str;

    #[test]
    fn return_the_given_expression() {
        let ast = parse_str("fn function(mut foo: String) {}").unwrap();
        let arg = first_arg_ident(&ast);
        let expected = expr("bar()");
        let mut resolver = HashMap::new();

        resolver.insert("foo".to_string(), &expected);

        assert_eq!(expected, (&resolver).resolve(&arg).unwrap().into_owned())
    }

    #[test]
    fn return_none_for_unknown_argument() {
        let ast = "fn function(mut fix: String) {}".ast();
        let arg = first_arg_ident(&ast);

        assert!(EmptyResolver.resolve(&arg).is_none())
    }
}
