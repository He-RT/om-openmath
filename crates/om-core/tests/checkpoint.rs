//! Shared expression data never evaluates definitions or loses underlying numeric bit patterns.
use om_core::checkpoint::{ExprLimits, decode_expressions, encode_expressions};
use om_core::{BUILTIN as B, Expr, Interrupt, Symbol};
use om_num::{Number, Real};
#[test]
fn shared_raw_expressions_and_non_symbol_heads_roundtrip() {
    let part = Expr::normal(
        Expr::sym(Symbol::intern("持久_é")),
        [Expr::rational(1, 3), Expr::string("原文e\u{301}\n🙂")],
    );
    let raw = Expr::normal(part.clone(), [part.clone(), part.clone(), Expr::int(7)]);
    let roots = [
        raw,
        part,
        Expr::call(B::SET, [Expr::sym(Symbol::intern("a")), Expr::int(5)]),
    ];
    let bytes = encode_expressions(&roots, ExprLimits::default(), &Interrupt::default()).unwrap();
    let decoded = decode_expressions(&bytes, ExprLimits::default(), &Interrupt::default()).unwrap();
    assert_eq!(decoded, roots);
    assert_eq!(
        encode_expressions(&decoded, ExprLimits::default(), &Interrupt::default()).unwrap(),
        bytes
    );
}
#[test]
fn signed_zero_atoms_are_not_collapsed_by_value_equality_deduplication() {
    let roots = [
        Expr::number(Number::Real(Real::Machine(0.0))),
        Expr::number(Number::Real(Real::Machine(-0.0))),
    ];
    assert_eq!(roots[0], roots[1]);
    let bytes = encode_expressions(&roots, ExprLimits::default(), &Interrupt::default()).unwrap();
    let decoded = decode_expressions(&bytes, ExprLimits::default(), &Interrupt::default()).unwrap();
    for (index, expr) in decoded.iter().enumerate() {
        let Some(Number::Real(Real::Machine(value))) = expr.as_number() else {
            panic!("numeric category changed")
        };
        assert_eq!(value.to_bits(), if index == 0 { 0 } else { 1u64 << 63 });
    }
}
#[test]
fn version_graph_indices_depth_count_size_and_abort_reject_partial_restore() {
    let root = Expr::call(B::PLUS, [Expr::int(1), Expr::int(2)]);
    let limits = ExprLimits::default();
    let ctx = Interrupt::default();
    let bytes = encode_expressions(&[root], limits, &ctx).unwrap();
    for n in 0..bytes.len() {
        assert!(decode_expressions(&bytes[..n], limits, &ctx).is_err());
    }
    let mut future = bytes.clone();
    future[4] = 2;
    assert!(decode_expressions(&future, limits, &ctx).is_err());
    assert!(
        decode_expressions(
            &bytes,
            ExprLimits {
                max_nodes: 1,
                ..limits
            },
            &ctx
        )
        .is_err()
    );
    assert!(
        decode_expressions(
            &bytes,
            ExprLimits {
                max_depth: 1,
                ..limits
            },
            &ctx
        )
        .is_err()
    );
    let mut corrupt = bytes.clone();
    let len = corrupt.len();
    corrupt[len - 4..].copy_from_slice(&u32::MAX.to_le_bytes());
    assert!(decode_expressions(&corrupt, limits, &ctx).is_err());
    ctx.flag.store(true, std::sync::atomic::Ordering::Relaxed);
    assert!(decode_expressions(&bytes, limits, &ctx).is_err());
}

#[test]
fn deeply_shared_subtrees_stay_linear_in_the_number_of_actual_nodes() {
    let mut root = Expr::int(1);
    for _ in 0..100 {
        root = Expr::call(Symbol::intern("unexecuted"), [root.clone(), root.clone()]);
    }
    let bytes = encode_expressions(&[root], ExprLimits::default(), &Interrupt::default()).unwrap();
    assert!(bytes.len() < 8192);
    let decoded = decode_expressions(&bytes, ExprLimits::default(), &Interrupt::default()).unwrap();
    assert_eq!(
        encode_expressions(&decoded, ExprLimits::default(), &Interrupt::default()).unwrap(),
        bytes
    );
}

#[test]
fn invalid_utf8_builtin_identity_orphan_nodes_and_forward_edges_are_closed() {
    let limits = ExprLimits::default();
    let ctx = Interrupt::default();
    let bytes = encode_expressions(&[Expr::sym(B::PLUS)], limits, &ctx).unwrap();
    let mut wrong = bytes.clone();
    wrong[13] = 2;
    assert!(decode_expressions(&wrong, limits, &ctx).is_err());
    let mut utf8 = bytes.clone();
    utf8[18] = 255;
    assert!(decode_expressions(&utf8, limits, &ctx).is_err());
    let mut orphan = bytes.clone();
    orphan[5..9].copy_from_slice(&0u32.to_le_bytes());
    orphan.truncate(orphan.len() - 4);
    assert!(decode_expressions(&orphan, limits, &ctx).is_err());
    let mut forward = b"OMEX\x01".to_vec();
    forward.extend(1u32.to_le_bytes());
    forward.extend(1u32.to_le_bytes());
    forward.push(4);
    forward.extend(0u32.to_le_bytes());
    forward.extend(0u32.to_le_bytes());
    forward.extend(0u32.to_le_bytes());
    assert!(decode_expressions(&forward, limits, &ctx).is_err());
}

#[test]
fn seeded_malformed_graphs_fail_without_panicking_or_unbounded_node_allocation() {
    let mut random = om_num::rng::SplitMix64::new(97);
    let limits = ExprLimits {
        max_bytes: 1024,
        max_nodes: 64,
        max_roots: 64,
        max_edges: 128,
        ..ExprLimits::default()
    };
    for n in 0..512 {
        let length = (n % 96) as usize;
        let mut bytes = vec![0; length];
        for byte in &mut bytes {
            *byte = random.next_u64() as u8;
        }
        if length >= 13 {
            bytes[..5].copy_from_slice(b"OMEX\x01");
        }
        let result =
            std::panic::catch_unwind(|| decode_expressions(&bytes, limits, &Interrupt::default()));
        assert!(result.is_ok());
        assert!(result.unwrap().is_err());
    }
}
