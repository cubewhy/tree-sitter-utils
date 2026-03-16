# tree-sitter-combinator

A composable, parser-combinator-style abstraction over tree-sitter node
dispatch. Instead of writing ad-hoc `loop { match node.kind() { ... } }`
patterns, you build a chain of typed, zero-cost `Handler` combinators that
express "how to map a syntax-tree node (plus arbitrary context) to an output
value". The crate is fully language-agnostic; all grammar knowledge lives in
the consumer crate.

---

## Quick start

Add the dependency:

```toml
[dependencies]
tree-sitter-combinator = "0.1"
tree-sitter = "0.23"
# plus your grammar crate, e.g. tree-sitter-python = "0.23"
```

Build a handler chain for a fictional language with four node kinds:

```rust
use tree_sitter_combinator::{handler_fn, never, HandlerExt, Input};

// Context your consumer crate supplies to every handler.
struct MyCtx<'a> {
    source: &'a str,
}

// Label nodes from a fictional grammar.
fn make_labeller<'a>() -> impl tree_sitter_combinator::Handler<&'a MyCtx<'a>, String> {
    // 1. Handle "func_decl" and "lambda" the same way.
    let callable = handler_fn(|_: Input<&MyCtx<'_>>| "callable".to_owned())
        .for_kinds(&["func_decl", "lambda"]);

    // 2. Handle "identifier" only when it sits inside a call expression.
    let ident_in_call = handler_fn(|inp: Input<&MyCtx<'_>>| {
        format!("ident-in-call:{}", inp.node.kind())
    })
    .for_kinds(&["identifier"])
    .when(tree_sitter_combinator::has_parent_kind("call_expr"));

    // 3. Climb to the nearest enclosing "block" for anything else.
    let block_climber = (|inp: Input<&MyCtx<'_>>| -> Option<String> {
        (inp.node.kind() == "block").then(|| "inside block".to_owned())
    })
    .climb(&["module"]);

    // Chain: try each in order, fall back to "unknown".
    callable
        .or(ident_in_call)
        .or(block_climber)
        .or(handler_fn(|inp: Input<&MyCtx<'_>>| {
            format!("unknown:{}", inp.node.kind())
        }))
}
```

---

## Illustrative example — Java-specific code lives in the consumer crate, not here

The original motivation was eliminating patterns like:

```java
// Java consumer crate (NOT part of tree-sitter-combinator)
private String determineLocation(Node node, Context ctx) {
    while (node != null) {
        switch (node.getType()) {
            case "method_declaration":
                return labelMethod(node, ctx);
            case "class_declaration":
                return labelClass(node, ctx);
            case "lambda_expression":
                return "lambda";
        }
        node = node.getParent();
    }
    return null;
}
```

With `tree-sitter-combinator` the same logic in the Java consumer crate collapses to:

```rust
// Java consumer crate — grammar strings stay here, NOT in tree-sitter-combinator.
use tree_sitter_combinator::{handler_fn, never, HandlerExt, Input};

fn make_location_handler() -> impl tree_sitter_combinator::Handler<MyJavaCtx, String> {
    handler_fn(|inp: Input<MyJavaCtx>| label_method(&inp.node, &inp.ctx))
        .for_kinds(&["method_declaration"])
        .or(
            handler_fn(|inp: Input<MyJavaCtx>| label_class(&inp.node, &inp.ctx))
                .for_kinds(&["class_declaration"]),
        )
        .or(
            handler_fn(|_: Input<MyJavaCtx>| "lambda".to_owned())
                .for_kinds(&["lambda_expression"]),
        )
        .climb(&["program"]) // ascend until one of the above matches
}
// type MyJavaCtx = ();
// fn label_method(_: &tree_sitter::Node<'_>, _: &()) -> String { String::new() }
// fn label_class(_: &tree_sitter::Node<'_>, _: &()) -> String { String::new() }
```

The handler is built once, stored cheaply (no heap allocation in the hot
path), and called with a single `handler.handle(input)` per node.

---

## Combinator reference

| Combinator | Signature sketch | Semantics |
|---|---|---|
| `.or(other)` | `(H, H2) -> Or<H, H2>` | Try `self`; on `None`, try `other`. |
| `.when(pred)` | `(H, P: NodePredicate) -> When<H, P>` | Run `self` only when `pred` returns `true`. |
| `.for_kinds(kinds)` | `(H, &'static [&'static str]) -> When<H, KindIs>` | Sugar for `.when(kind_is(kinds))`. |
| `.map(f)` | `(H, Fn(R)->R2) -> Map<H, F, R>` | Transform a `Some(out)` result. |
| `.map_input(f)` | `(H, Fn(Input)->Input) -> MapInput<H, F>` | Transform the `Input` before passing it to `self`. |
| `.and_then(f)` | `(H, Fn(Input,R)->Option<R2>) -> AndThen<H,F,R>` | Flat-map: feed `(input, out)` into `f` on success. |
| `.climb(stop_kinds)` | `(H, &'static [&'static str]) -> Climb<H>` | On `None`, walk `parent()` and retry `self` until a stop-kind or root. |
| `.or_else_climb(other, stop_kinds)` | `(H, H2, &'static [&'static str]) -> OrElseClimb<H,H2>` | Try `self`; on `None`, try `other` on each ancestor up to stop-kind. |
| `.boxed()` | `H -> BoxedHandler<Ctx, R>` | Erase the type for dynamic dispatch (heap-allocates). |

Free-function constructors:

| Function | Returns | Semantics |
|---|---|---|
| `handler_fn(f)` | `HandlerFn<F>` | Wrap an infallible `Fn(Input)->R`; always returns `Some`. |
| `never()` | `Never<Ctx, R>` | Always returns `None`. |
| `always(value)` | `Always<R>` | Always returns `Some(value.clone())`. |
| `dispatch_on_kind(table)` | `DispatchOnKind<Ctx, R>` | Static kind→handler lookup table. |
| `first_of(handlers)` | `FirstOf<Ctx, R>` | Try a `Vec<BoxedHandler>` in order; return first `Some`. |

---

## MSRV and dependency policy

- **Minimum Supported Rust Version**: 1.75 (stable, no nightly features).
- **Mandatory dependency**: `tree-sitter = "0.23"`.
- **No unsafe code** except `unsafe impl Send/Sync` on predicate structs that
  wrap `&'static` data (which is trivially safe).
- Optional utility dependencies may be added behind feature flags in future
  releases without breaking the MSRV guarantee.
