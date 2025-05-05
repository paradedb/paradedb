use sqlparser::ast::{
    Expr, Ident, ObjectName, ObjectNamePart, Query, Statement, TableAlias, TableFactor, Visit,
    Visitor,
};
use sqlparser::dialect::PostgreSqlDialect;
use sqlparser::parser::Parser;
use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;

pub fn main() {
    let sql = r#"
        WITH foo AS (select * from a)
        SELECT a.title, b.severity, foo.message, paradedb.score(b.id)
          FROM a
          JOIN b ON a.id = b.id
          JOIN foo ON b.id = foo.id
          WHERE a.title = 'beer' and b.severity > 42;
    "#;

    let ast = Parser::parse_sql(&PostgreSqlDialect {}, sql).expect("input sql should be parseable");

    let tables = extract_query_data(&ast[0]);
    eprintln!("tables={:#?}", tables);
}

enum Action {
    CreateTable { name: String, actions: Vec<Action> },
    AddField { name: String, sqltype: String },
}

fn plan(basics: &QueryBasics) -> Vec<Action> {
    let mut actions = Vec::new();

    let mut ctes = basics
        .ctes
        .iter()
        .map(|alias| alias.name.to_string())
        .collect::<HashSet<_>>();

    let relations = basics
        .relations
        .iter()
        .map(|ident| ident.to_string())
        .collect::<HashSet<_>>();

    let identifiers = basics
        .identifiers
        .iter()
        .map(|ident| {
            let table_name = ident[0].to_string();
            let field_name = ident[1].to_string();
            (table_name, field_name)
        })
        .collect::<HashMap<_, _>>();

    actions
}

#[derive(Default, Debug)]
struct QueryBasics {
    ctes: Vec<TableAlias>,
    relations: Vec<Ident>,
    identifiers: Vec<Vec<Ident>>,
    expressions: Vec<Expr>,
}

impl Visitor for QueryBasics {
    type Break = ();

    fn pre_visit_query(&mut self, query: &Query) -> ControlFlow<Self::Break> {
        if let Some(with) = &query.with {
            self.ctes
                .extend(with.cte_tables.iter().map(|cte| cte.alias.clone()))
        }

        ControlFlow::Continue(())
    }

    fn pre_visit_relation(&mut self, relation: &ObjectName) -> ControlFlow<Self::Break> {
        self.relations.extend(relation.0.iter().map(|i| match i {
            ObjectNamePart::Identifier(i) => i.clone(),
        }));
        ControlFlow::Continue(())
    }

    fn pre_visit_table_factor(&mut self, table_factor: &TableFactor) -> ControlFlow<Self::Break> {
        ControlFlow::Continue(())
    }

    fn pre_visit_expr(&mut self, expr: &Expr) -> ControlFlow<Self::Break> {
        match expr {
            Expr::Identifier(i) => self.identifiers.push(vec![i.clone()]),
            Expr::CompoundIdentifier(ci) => self.identifiers.push(ci.clone()),

            expr => self.expressions.push(expr.clone()),
        }
        ControlFlow::Continue(())
    }
}
fn extract_query_data(stmt: &Statement) -> QueryBasics {
    let mut v = QueryBasics::default();
    stmt.visit(&mut v);
    v
}
