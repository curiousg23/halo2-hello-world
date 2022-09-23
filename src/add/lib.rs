/// Constructs the Add chip.
use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{Chip, Layouter, Region},
    plonk::{Advice, Column, ConstraintSystem, Error, Selector},
    poly::Rotation,
};

/// Interface for the AddInstruction.
pub(crate) trait AddInstructions<F: FieldExt>: Chip<F> {
    /// Variable representing a number.
    type Num;

    /// Returns `c = a + b`.
    fn add(
        &self,
        layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error>;
}

/// Config for the add chip.
#[derive(Clone, Debug)]
pub(crate) struct AddConfig {
    /// One advice column for the instruction.
    advice: Column<Advice>,
    /// Selector for the add instruction.
    s_add: Selector,
}

/// A chip for the add functionality.
pub(crate) struct AddChip<F: FieldExt> {
    config: AddConfig,
    _marker: PhantomData<F>,
}

// Implementations for the add chip below.

impl<F: FieldExt> Chip<F> for AddChip<F> {
    type Config = AddConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> AddChip<F> {
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
        let s_add = meta.selector();

        // Define our addition gate.
        meta.create_gate("add", |meta| {
            // We want three advice cells and a selector cell.
            // | a0  | s_add |
            // |-----|-------|
            // | lhs | s_add |
            // | rhs |       |
            // | out |       |
            let lhs = meta.query_advice(advice, Rotation::cur());
            let rhs = meta.query_advice(advice, Rotation::next());
            // ::next() is not an iterator.
            let out = meta.query_advice(advice, Rotation(2));
            let s_add = meta.query_selector(s_add);

            // When s_add = 0, any value is allowed in lhs, rhs, out.
            // When s_add != 0, lhs + rhs = out.
            vec![s_add * (lhs + rhs - out)]
        });

        AddConfig { advice, s_add }
    }
}

impl<F: FieldExt> AddInstructions<F> for AddChip<F> {
    type Num = crate::Number<F>;

    fn add(
        &self,
        mut layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error> {
        let config = self.config();

        layouter.assign_region(
            || "add",
            |mut region: Region<'_, F>| {
                config.s_add.enable(&mut region, 0)?;

                a.0.copy_advice(|| "lhs", &mut region, config.advice, 0)?;
                b.0.copy_advice(|| "rhs", &mut region, config.advice, 1)?;

                let value = a.0.value().copied() + b.0.value().copied();

                region
                    .assign_advice(|| "lhs + rhs", config.advice, 2, || value)
                    .map(crate::Number)
            },
        )
    }
}
