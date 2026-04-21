use crate::evaluator::logic::bytecode::compile::analysis::gvn::optimize_vir_gvn;
use crate::evaluator::logic::bytecode::compile::optimize::compact::compact_constants;
use crate::evaluator::logic::bytecode::compile::optimize::dce::eliminate_dead_code;
use crate::evaluator::logic::bytecode::compile::optimize::fusion::fuse_instructions;
use crate::evaluator::logic::bytecode::compile::optimize::helper::ConstantPool;
use crate::evaluator::logic::bytecode::compile::optimize::power_chain::optimize_power_chains;
use crate::evaluator::logic::bytecode::compile::vir::{VInstruction, VReg};
use crate::evaluator::logic::bytecode::compile::{FnOp, Instruction};
use rustc_hash::FxHashMap;

#[cfg(test)]
#[allow(
    clippy::panic,
    reason = "Test-only module uses panics for expressive failure messages"
)]
mod unit_tests {
    use super::*;

    #[test]
    fn test_gvn_constant_folding_basic() {
        let mut constants = vec![2.0, 3.0];
        let mut const_map = FxHashMap::default();
        const_map.insert(2.0_f64.to_bits(), 0);
        const_map.insert(3.0_f64.to_bits(), 1);

        // 2 + 3 -> 5
        let mut vinstrs = vec![VInstruction::Add2 {
            dest: VReg::Temp(0),
            a: VReg::Const(0),
            b: VReg::Const(1),
        }];

        let mut final_vreg = Some(VReg::Temp(0));
        optimize_vir_gvn(
            &mut vinstrs,
            &mut final_vreg,
            &mut constants,
            &mut const_map,
            0,
        );

        // Should be folded to a constant
        assert!(vinstrs.is_empty());
        if let Some(VReg::Const(idx)) = final_vreg {
            #[allow(
                clippy::float_cmp,
                reason = "Exact bitwise comparison for constant folding test"
            )]
            {
                assert_eq!(constants[idx as usize], 5.0);
            }
        } else {
            panic!("Expected final_vreg to be a constant, got {final_vreg:?}");
        }
    }

    #[test]
    fn test_gvn_builtin_folding() {
        let mut constants = vec![0.0];
        let mut const_map = FxHashMap::default();
        const_map.insert(0.0_f64.to_bits(), 0);

        // sin(0) -> 0
        let mut vinstrs = vec![VInstruction::Builtin1 {
            dest: VReg::Temp(0),
            op: FnOp::Sin,
            arg: VReg::Const(0),
        }];

        let mut final_vreg = Some(VReg::Temp(0));
        optimize_vir_gvn(
            &mut vinstrs,
            &mut final_vreg,
            &mut constants,
            &mut const_map,
            0,
        );

        assert!(vinstrs.is_empty());
        if let Some(VReg::Const(idx)) = final_vreg {
            #[allow(
                clippy::float_cmp,
                reason = "Exact bitwise comparison for constant folding test"
            )]
            {
                assert_eq!(constants[idx as usize], 0.0);
            }
        } else {
            panic!("Expected final_vreg to be a constant");
        }
    }

    #[test]
    fn test_power_chain_optimization() {
        // x^2, x^3
        let mut instrs = vec![
            Instruction::Square { dest: 10, src: 0 },
            Instruction::Cube { dest: 11, src: 0 },
        ];

        optimize_power_chains(&mut instrs);

        // Cube(x) should become Mul(Square(x), x)
        assert_eq!(instrs.len(), 2);
        assert_eq!(instrs[0], Instruction::Square { dest: 10, src: 0 });
        assert_eq!(
            instrs[1],
            Instruction::Mul {
                dest: 11,
                a: 10,
                b: 0
            }
        );
    }

    #[test]
    fn test_power_chain_overwritten_base() {
        // R10 = x^2
        // R0 = y (overwrites base)
        // R11 = x^3 (cannot use R10 anymore)
        let mut instrs = vec![
            Instruction::Square { dest: 10, src: 0 },
            Instruction::Copy { dest: 0, src: 1 }, // R0 = R1
            Instruction::Cube { dest: 11, src: 0 },
        ];

        optimize_power_chains(&mut instrs);

        // The Cube should NOT be optimized to use R10 because R0 was overwritten.
        assert_eq!(instrs[2], Instruction::Cube { dest: 11, src: 0 });
    }

    #[test]
    fn test_gvn_identity_preservation() {
        let mut constants = vec![1e-17]; // Smaller than EPSILON
        let mut const_map = FxHashMap::default();
        const_map.insert(1e-17_f64.to_bits(), 0);

        // x + 1e-17 should NOT be folded to x
        let mut vinstrs = vec![VInstruction::Add2 {
            dest: VReg::Temp(0),
            a: VReg::Temp(1),
            b: VReg::Const(0),
        }];

        let mut final_vreg = Some(VReg::Temp(0));
        optimize_vir_gvn(
            &mut vinstrs,
            &mut final_vreg,
            &mut constants,
            &mut const_map,
            0,
        );

        // Should NOT be empty (not simplified away)
        assert!(!vinstrs.is_empty());
    }

    #[test]
    fn test_gvn_mul2_preserves_nan_when_zero_times_nan() {
        let mut constants = vec![0.0, f64::NAN];
        let mut const_map = FxHashMap::default();
        const_map.insert(0.0_f64.to_bits(), 0);
        const_map.insert(f64::NAN.to_bits(), 1);

        let mut vinstrs = vec![VInstruction::Mul2 {
            dest: VReg::Temp(0),
            a: VReg::Const(0),
            b: VReg::Const(1),
        }];

        let mut final_vreg = Some(VReg::Temp(0));
        optimize_vir_gvn(
            &mut vinstrs,
            &mut final_vreg,
            &mut constants,
            &mut const_map,
            0,
        );

        assert!(vinstrs.is_empty());
        match final_vreg {
            Some(VReg::Const(idx)) => assert!(constants[idx as usize].is_nan()),
            _ => panic!("Expected final_vreg to fold to a NaN constant"),
        }
    }

    #[test]
    fn test_gvn_div_zero_by_zero_preserves_nan() {
        let mut constants = vec![0.0];
        let mut const_map = FxHashMap::default();
        const_map.insert(0.0_f64.to_bits(), 0);

        let mut vinstrs = vec![VInstruction::Div {
            dest: VReg::Temp(0),
            num: VReg::Const(0),
            den: VReg::Const(0),
        }];

        let mut final_vreg = Some(VReg::Temp(0));
        optimize_vir_gvn(
            &mut vinstrs,
            &mut final_vreg,
            &mut constants,
            &mut const_map,
            0,
        );

        assert!(vinstrs.is_empty());
        match final_vreg {
            Some(VReg::Const(idx)) => assert!(constants[idx as usize].is_nan()),
            _ => panic!("Expected final_vreg to fold to a NaN constant"),
        }
    }

    #[test]
    fn test_power_chain_squaring_even_powers() {
        // x^3, x^6
        // x^6 should become Square(x^3)
        let mut instrs = vec![
            Instruction::Cube { dest: 10, src: 0 },
            Instruction::Powi {
                dest: 11,
                src: 0,
                n: 6,
            },
        ];

        optimize_power_chains(&mut instrs);

        assert_eq!(instrs[1], Instruction::Square { dest: 11, src: 10 });
    }

    #[test]
    fn test_dce_basic() {
        let instrs = vec![
            Instruction::Add {
                dest: 10,
                a: 1,
                b: 2,
            }, // Dead
            Instruction::Mul {
                dest: 0,
                a: 1,
                b: 2,
            }, // Live (output)
        ];
        let mut arg_pool = vec![];
        let mut use_count = vec![0; 20];

        let out = eliminate_dead_code(
            instrs,
            &mut arg_pool,
            &mut use_count,
            2,  // param_count
            0,  // const_count
            10, // max_reg_idx
            0,  // output_reg
            &mut crate::evaluator::logic::bytecode::compile::optimize::dce::DceScratch::new(),
        );

        assert_eq!(out.len(), 1);
        assert_eq!(
            out[0],
            Instruction::Mul {
                dest: 0,
                a: 1,
                b: 2
            }
        );
    }

    #[test]
    fn test_fusion_fma() {
        let instrs = vec![
            Instruction::Mul {
                dest: 10,
                a: 1,
                b: 2,
            },
            Instruction::Add {
                dest: 0,
                a: 10,
                b: 11,
            },
        ];
        let use_count = vec![0, 2, 2, 0, 0, 0, 0, 0, 0, 0, 1, 1]; // R10 used once

        let mut constants = vec![];
        let mut pool = ConstantPool::with_index(&mut constants, FxHashMap::default(), 0);
        let (fused, _) = fuse_instructions(&instrs, &mut pool, &use_count, &[]);

        assert_eq!(fused.len(), 1);
        if let Instruction::MulAdd { dest, a, b, c } = fused[0] {
            assert_eq!(dest, 0);
            assert_eq!(a, 1);
            assert_eq!(b, 2);
            assert_eq!(c, 11);
        } else {
            panic!("Expected MulAdd fusion, got {:?}", fused[0]);
        }
    }

    #[test]
    fn test_fusion_add_const_chain_to_copy_when_constants_cancel() {
        let instrs = vec![
            Instruction::Add {
                dest: 10,
                a: 1,
                b: 2, // Constant 2.5
            },
            Instruction::Add {
                dest: 0,
                a: 10,
                b: 3, // Constant -2.5
            },
        ];
        let use_count = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1];

        let mut constants = vec![2.5, -2.5];
        let mut pool = ConstantPool::with_index(&mut constants, FxHashMap::default(), 2);
        let (fused, _) = fuse_instructions(&instrs, &mut pool, &use_count, &[]);

        assert_eq!(fused, vec![Instruction::Copy { dest: 0, src: 1 }]);
    }

    #[test]
    fn test_fusion_mul_const_chain_to_load_const_when_product_is_zero() {
        let instrs = vec![
            Instruction::Mul {
                dest: 10,
                a: 1,
                b: 2, // Constant 0.0
            },
            Instruction::Mul {
                dest: 0,
                a: 10,
                b: 3, // Constant 4.0
            },
        ];
        let use_count = vec![0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 1];

        let mut constants = vec![0.0, 4.0];
        let mut index = FxHashMap::default();
        index.insert(0.0_f64.to_bits(), 0);
        index.insert(4.0_f64.to_bits(), 1);
        let mut pool = ConstantPool::with_index(&mut constants, index, 2);
        let (fused, _) = fuse_instructions(&instrs, &mut pool, &use_count, &[]);

        assert_eq!(
            fused,
            vec![Instruction::Copy {
                dest: 0,
                src: 2, // Constant 0.0
            }]
        );
    }

    #[test]
    fn test_constant_pool_reconstruction_keeps_first_duplicate_index() {
        let mut constants = vec![3.0, 7.0, 3.0];
        let mut index = FxHashMap::default();
        index.insert(3.0_f64.to_bits(), 0);
        index.insert(7.0_f64.to_bits(), 1);
        let mut pool = ConstantPool::with_index(&mut constants, index, 2);

        assert_eq!(pool.get_or_insert(3.0), 2);
        assert_eq!(pool.get_or_insert(7.0), 3);
    }

    #[test]
    fn test_dce_forwards_multi_use_copy_when_source_stays_live() {
        let instrs = vec![
            Instruction::Copy { dest: 5, src: 3 },
            Instruction::Add {
                dest: 6,
                a: 5,
                b: 1,
            },
            Instruction::Mul {
                dest: 0,
                a: 6,
                b: 5,
            },
        ];
        let mut arg_pool = vec![];
        let mut use_count = vec![0; 8];

        let out = eliminate_dead_code(
            instrs,
            &mut arg_pool,
            &mut use_count,
            3,
            0,
            6,
            0,
            &mut crate::evaluator::logic::bytecode::compile::optimize::dce::DceScratch::new(),
        );

        assert_eq!(
            out,
            vec![
                Instruction::Add {
                    dest: 6,
                    a: 3,
                    b: 1
                },
                Instruction::Mul {
                    dest: 0,
                    a: 6,
                    b: 3
                },
            ]
        );
    }

    #[test]
    fn test_compact_constants_preserves_output_register_reads() {
        let mut instructions = vec![
            Instruction::Copy {
                dest: 2,
                src: 1, // Constant 20.0
            },
            Instruction::Add {
                dest: 3,
                a: 2,
                b: 1, // Constant 20.0
            },
        ];
        let mut constants = vec![10.0, 20.0];

        let mut arg_pool = vec![];

        let (out, rc, result_reg) = compact_constants(
            std::mem::take(&mut instructions),
            &mut constants,
            &mut arg_pool,
            0, // param_count
            2, // old_const_count
            3, // output_reg (Temp 3)
        );

        // 10.0 (reg 0) is unused and should be removed.
        // 20.0 (reg 1) becomes reg 0.
        // Temp 2 becomes reg 1.
        // Temp 3 becomes reg 2.
        assert_eq!(
            out,
            vec![
                Instruction::Copy { dest: 1, src: 0 },
                Instruction::Add {
                    dest: 2,
                    a: 1,
                    b: 0
                },
            ]
        );
        assert_eq!(constants, vec![20.0]);
        assert_eq!(rc, 3);
        assert_eq!(result_reg, 2);
    }

    #[test]
    fn test_gvn_preserves_vreg_identities() {
        let mut constants = vec![0.0, 1.0];
        let mut const_map = FxHashMap::default();
        const_map.insert(0.0_f64.to_bits(), 0);
        const_map.insert(1.0_f64.to_bits(), 1);

        // x - x should NOT be folded if x is a VReg
        {
            let mut vinstrs = vec![VInstruction::Sub {
                dest: VReg::Temp(0),
                a: VReg::Temp(1),
                b: VReg::Temp(1),
            }];
            let mut final_vreg = Some(VReg::Temp(0));
            optimize_vir_gvn(
                &mut vinstrs,
                &mut final_vreg,
                &mut constants,
                &mut const_map,
                0,
            );
            assert!(!vinstrs.is_empty());
        }

        // 0 * x should NOT be folded if x is a VReg
        {
            let mut vinstrs = vec![VInstruction::Mul2 {
                dest: VReg::Temp(0),
                a: VReg::Const(0),
                b: VReg::Temp(1),
            }];
            let mut final_vreg = Some(VReg::Temp(0));
            optimize_vir_gvn(
                &mut vinstrs,
                &mut final_vreg,
                &mut constants,
                &mut const_map,
                0,
            );
            assert!(!vinstrs.is_empty());
        }

        // x / x should NOT be folded if x is a VReg
        {
            let mut vinstrs = vec![VInstruction::Div {
                dest: VReg::Temp(0),
                num: VReg::Temp(1),
                den: VReg::Temp(1),
            }];
            let mut final_vreg = Some(VReg::Temp(0));
            optimize_vir_gvn(
                &mut vinstrs,
                &mut final_vreg,
                &mut constants,
                &mut const_map,
                0,
            );
            assert!(!vinstrs.is_empty());
        }
    }

    #[test]
    fn test_gvn_special_function_rounding_folding() {
        let mut constants = vec![1.5, 2.0];
        let mut const_map = FxHashMap::default();
        const_map.insert(1.5_f64.to_bits(), 0);
        const_map.insert(2.0_f64.to_bits(), 1);

        // BesselJ(1.5, 2.0) should fold to BesselJ(2, 2.0)
        let mut vinstrs = vec![VInstruction::Builtin2 {
            dest: VReg::Temp(0),
            op: FnOp::BesselJ,
            arg1: VReg::Const(0),
            arg2: VReg::Const(1),
        }];

        let mut final_vreg = Some(VReg::Temp(0));
        optimize_vir_gvn(
            &mut vinstrs,
            &mut final_vreg,
            &mut constants,
            &mut const_map,
            0,
        );

        assert!(vinstrs.is_empty());
        if let Some(VReg::Const(idx)) = final_vreg {
            let val = constants[idx as usize];
            let expected = crate::math::bessel_j(2, 2.0);
            #[allow(
                clippy::float_cmp,
                reason = "Exact bitwise comparison for constant folding verification"
            )]
            {
                assert_eq!(val, expected);
            }
        } else {
            panic!("Expected BesselJ(1.5, 2.0) to fold to constant BesselJ(2, 2.0)");
        }
    }

    /// Regression test for chained copy forwarding.
    ///
    /// Without the fix, the chain `Copy R5=R2(param)` → `Copy R6=R5(temp)`
    /// would forward R6→R5 (one hop), then drop both Copies, leaving R5
    /// undefined when instructions still reference it.
    ///
    /// The correct behavior is to resolve R6 all the way to R2 (the immutable
    /// root) so both Copies can be safely dropped.
    #[test]
    fn test_dce_chained_copy_forwarding_to_param() {
        // Register layout: R0=param0, R1=param1, R2=param2, R3=param3
        // R5 = Copy(R2), R6 = Copy(R5), R7 = Add(R6, R3) → output in R0
        let instrs = vec![
            Instruction::Copy { dest: 5, src: 2 },
            Instruction::Copy { dest: 6, src: 5 },
            Instruction::Add {
                dest: 0,
                a: 6,
                b: 3,
            },
        ];
        let mut arg_pool = vec![];
        let mut use_count = vec![0; 8];

        let out = eliminate_dead_code(
            instrs,
            &mut arg_pool,
            &mut use_count,
            4, // param_count (R0..R3 are params)
            0, // const_count
            6, // max_reg_idx
            0, // output_reg
            &mut crate::evaluator::logic::bytecode::compile::optimize::dce::DceScratch::new(),
        );

        // Both Copies should be eliminated and R6 should resolve directly to R2
        assert_eq!(
            out,
            vec![Instruction::Add {
                dest: 0,
                a: 2,
                b: 3,
            }]
        );
    }

    /// Tests that a temp-to-temp copy chain where the root is also a temp
    /// does NOT produce a stale forwarding. The intermediate Copy must
    /// survive to preserve the value.
    #[test]
    fn test_dce_chained_copy_temp_to_temp_root_preserved() {
        // R0..R1 = params, R2 = Add(R0, R1), R3 = Copy(R2), R4 = Copy(R3)
        // output = Mul(R4, R0) in R5 → output_reg = 5
        let instrs = vec![
            Instruction::Add {
                dest: 2,
                a: 0,
                b: 1,
            },
            Instruction::Copy { dest: 3, src: 2 },
            Instruction::Copy { dest: 4, src: 3 },
            Instruction::Mul {
                dest: 5,
                a: 4,
                b: 0,
            },
        ];
        let mut arg_pool = vec![];
        let mut use_count = vec![0; 8];

        let out = eliminate_dead_code(
            instrs,
            &mut arg_pool,
            &mut use_count,
            2, // param_count (R0, R1)
            0, // const_count
            5, // max_reg_idx
            5, // output_reg
            &mut crate::evaluator::logic::bytecode::compile::optimize::dce::DceScratch::new(),
        );

        // The first Copy (R3=R2) is forwarded and dropped.
        // The second Copy (R4=R3) cannot chain to R2 (temp root), so it
        // survives with its source rewritten: R4=R2.
        // The Mul reads R4 which has R2's value.
        // Total: Add, Copy(R4=R2), Mul — OR — Add, Mul(R2, R0) if both copies resolved.
        // Either way, the result must be correct (no undefined registers).
        for instr in &out {
            // Verify no instruction reads a register that isn't defined
            let mut dest_set = rustc_hash::FxHashSet::default();
            for out_instr in &out {
                out_instr.for_each_write(|d| {
                    dest_set.insert(d);
                });
            }
            instr.for_each_read(|r| {
                let is_param = r < 2;
                let is_defined = dest_set.contains(&r);
                assert!(
                    is_param || is_defined,
                    "Register R{r} is read but never defined (use-before-def bug)"
                );
            });
        }
    }
}
