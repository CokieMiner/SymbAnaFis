//! Fixed function-domain and arity edges.

use alloc::vec;

use crate::core::{Function, Symbol};

use super::super::{Expr, ExprView};

#[test]
fn lambert_branch_cells_keep_exact_domain_rules() {
    let x = Expr::symbol("builtin_lambert_branch_x");
    assert!(
        x.lambertw(0).same_node(
            &Function::builtin("lambert_w")
                .expect("the builtin alias is known")
                .call(vec![Expr::number(0), x.clone()])
                .expect("Lambert W accepts two arguments")
        )
    );
    assert!(x.lambertw(0).same_node(&x.lambertw(Expr::number(0))));
    assert!(matches!(
        Expr::number(0).lambertw(-1).view(),
        ExprView::Function { .. }
    ));
    assert!(matches!(
        x.lambertw(0.0).view(),
        ExprView::Function { args, .. } if args.len() == 2
    ));
    let symbol = Symbol::intern("builtin_lambert_branch_symbol");
    assert!(
        symbol.lambertw(-1).same_node(
            &Function::builtin("lambert_w")
                .expect("the builtin alias is known")
                .call(vec![Expr::number(-1), symbol.to_expr()])
                .expect("Lambert W accepts two arguments")
        )
    );
}

#[test]
fn principal_lambert_zero_is_the_only_proven_zero_cell() {
    let function = Function::builtin("lambert_w").expect("the builtin alias is known");
    assert!(
        function
            .call(vec![Expr::number(0), Expr::number(0)])
            .expect("Lambert W accepts two arguments")
            .is_zero_num()
    );
    assert!(function.call(vec![Expr::number(0)]).is_err());
}
