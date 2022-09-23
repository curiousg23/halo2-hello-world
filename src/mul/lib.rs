/// Constructs the Mul chip.
use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};

pub(crate) trait MulInstructions<F: FieldExt>: Chip<F> {
    /// Variable representing a number.
    type Num;

    /// Returns `c = a * b`.
    fn mul(
        &self,
        layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error>;
}

pub(crate) struct MulChip<F: FieldExt> {
    config: MulConfig,
    _marker: PhantomData<F>,
}

#[derive(Clone, Debug)]
pub(crate) struct MulConfig {
    /// Two advice columns for the instruction.
    advice: Column<Advice>,
    /// Selector for the multiply instruction.
    s_mul: Selector,
}

impl<F: FieldExt> Chip<F> for MulChip<F> {
    type Config = MulConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> MulChip<F> {
    pub(crate) fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    pub(crate) fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice>,
    ) -> <Self as Chip<F>>::Config {
        meta.enable_equality(advice);
        let s_mul = meta.selector();

        // Define our multiplication gate.
        meta.create_gate("mul", |meta| {
            // We want three advice cells and a selector cell.
            // | a0  | s_mul |
            // |-----|-------|
            // | lhs | s_mul |
            // | rhs |       |
            // | out |       |
            let lhs = meta.query_advice(advice, Rotation::cur());
            let rhs = meta.query_advice(advice, Rotation::next());
            let out = meta.query_advice(advice, Rotation(2));
            let s_mul = meta.query_selector(s_mul);

            // When s_add = 0, any value is allowed in lhs, rhs, out.
            // When s_add != 0, lhs * rhs = out.
            vec![s_mul * (lhs * rhs - out)]
        });

        MulConfig { advice, s_mul }
    }
}

impl<F: FieldExt> MulInstructions<F> for MulChip<F> {
    type Num = crate::Number<F>;

    fn mul(
        &self,
        mut layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error> {
        let config = self.config();

        layouter.assign_region(
            || "mul",
            |mut region: Region<'_, F>| {
                config.s_mul.enable(&mut region, 0)?;
                a.0.copy_advice(|| "lhs", &mut region, config.advice, 0)?;
                b.0.copy_advice(|| "rhs", &mut region, config.advice, 1)?;

                let value = a.0.value().copied() * b.0.value().copied();

                region
                    .assign_advice(|| "lhs * rhs", config.advice, 2, || value)
                    .map(crate::Number)
            },
        )
    }
}
