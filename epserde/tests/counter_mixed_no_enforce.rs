/*
 * Regression test for finding #3: behavioural asymmetry between
 * `enforce_repl(T)` and natural-replaceability for the
 * "T appears as both a direct field and inside a wrapper" pattern.
 *
 * Per `CLAUDE.md:103` and the design spec (lines 97-100), the
 * invariant
 *
 *     "a replaceable type parameter must not appear both as a field
 *      type and as a parameter of another field type"
 *
 * is "naturally lifted" — the new type-containment dispatch resolves
 * both syntactic positions to the same substituted form regardless
 * of whether T was naturally replaceable or forced via
 * `enforce_repl`.
 *
 * Currently false: the bound-skip in `gen_ser_deser_where_clauses`
 * (line 488) only fires for fields containing an `enforce_repl`
 * ident, NOT a naturally-replaceable one. So a struct that lists T
 * directly AND inside `A<T>` fails to compile with the Rust
 * #152409 `?` operator type mismatch — whereas adding
 * `#[epserde(enforce_repl(T))]` to the same struct makes it compile
 * (see `test_enforce_repl::test_enforce_repl_mixed_position`).
 *
 * Once the fix is in (either making the skip use the union
 * `repl_params`, or amending CLAUDE.md to require `enforce_repl`),
 * this test must either compile and round-trip or be deleted
 * alongside the doc change.
 */

use epserde::prelude::*;

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct A<T>(T);

#[derive(Epserde, Debug, PartialEq, Eq, Clone)]
struct WithoutEnforce<T>(T, A<T>);

#[test]
fn test_mixed_no_enforce() -> anyhow::Result<()> {
    let original: WithoutEnforce<Vec<u32>> = WithoutEnforce(vec![10, 20], A(vec![30, 40, 50]));
    let mut cursor = <AlignedCursor<Aligned16>>::new();
    unsafe { original.serialize(&mut cursor)? };

    cursor.set_position(0);
    let full = unsafe { <WithoutEnforce<Vec<u32>>>::deserialize_full(&mut cursor)? };
    assert_eq!(original, full);

    let _eps = unsafe { <WithoutEnforce<Vec<u32>>>::deserialize_eps(cursor.as_bytes())? };
    Ok(())
}
