use std::marker::PhantomData;

use halo2_proofs::{
    arithmetic::FieldExt,
    circuit::{AssignedCell, Chip, Layouter, SimpleFloorPlanner, Value},
    plonk::{Advice, Circuit, Column, ConstraintSystem, Error, Instance},
};

mod add;
mod mul;

use add::lib::{AddChip, AddConfig, AddInstructions};
use mul::lib::{MulChip, MulConfig, MulInstructions};

/// A variable representing a number.
#[derive(Clone)]
struct Number<F: FieldExt>(AssignedCell<F, F>);

trait SolutionInstructions<F: FieldExt>: AddInstructions<F> + MulInstructions<F> {
    /// Variable representing a number.
    type Num;

    /// Loads a number into the circuit as a private input.
    fn load_private(
        &self,
        layouter: impl Layouter<F>,
        a: Value<F>,
    ) -> Result<<Self as SolutionInstructions<F>>::Num, Error>;

    /// Loads a, b, c into the circuit.
    fn load_constants(
        &self,
        layouter: impl Layouter<F>,
    ) -> Result<[<Self as SolutionInstructions<F>>::Num; 3], Error>;

    /// Exposes a number as a public input to the circuit.
    fn expose_public(
        &self,
        layouter: impl Layouter<F>,
        num: <Self as SolutionInstructions<F>>::Num,
        row: usize,
    ) -> Result<(), Error>;

    /// Returns a * x^2 + b * x - c.
    fn solve_quadratic(
        &self,
        layouter: &mut impl Layouter<F>,
        a: <Self as SolutionInstructions<F>>::Num,
        b: <Self as SolutionInstructions<F>>::Num,
        c: <Self as SolutionInstructions<F>>::Num,
        x: <Self as SolutionInstructions<F>>::Num,
    ) -> Result<<Self as SolutionInstructions<F>>::Num, Error>;
}

struct SolutionChip<F: FieldExt> {
    config: SolutionConfig,
    _marker: PhantomData<F>,
}

#[derive(Clone, Debug)]
struct SolutionConfig {
    /// One column for the instruction.
    advice: Column<Advice>,
    /// One column for the instance variables (a, b, c).
    instance: Column<Instance>,
    /// Config for the `Add` chip.
    add_config: AddConfig,
    /// Config for the `Mul` chip.
    mul_config: MulConfig,
}

impl<F: FieldExt> Chip<F> for SolutionChip<F> {
    type Config = SolutionConfig;
    type Loaded = ();

    fn config(&self) -> &Self::Config {
        &self.config
    }

    fn loaded(&self) -> &Self::Loaded {
        &()
    }
}

impl<F: FieldExt> AddInstructions<F> for SolutionChip<F> {
    type Num = Number<F>;

    fn add(
        &self,
        layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error> {
        let config = self.config().add_config.clone();
        let add_chip = AddChip::<F>::construct(config);
        add_chip.add(layouter, a, b)
    }
}

impl<F: FieldExt> MulInstructions<F> for SolutionChip<F> {
    type Num = Number<F>;

    fn mul(
        &self,
        layouter: impl Layouter<F>,
        a: Self::Num,
        b: Self::Num,
    ) -> Result<Self::Num, Error> {
        let config = self.config().mul_config.clone();
        let mul_chip = MulChip::<F>::construct(config);
        mul_chip.mul(layouter, a, b)
    }
}

impl<F: FieldExt> SolutionChip<F> {
    fn construct(config: <Self as Chip<F>>::Config) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(
        meta: &mut ConstraintSystem<F>,
        advice: Column<Advice>,
        instance: Column<Instance>,
    ) -> <Self as Chip<F>>::Config {
        let add_config = AddChip::configure(meta, advice);
        let mul_config = MulChip::configure(meta, advice);
        meta.enable_equality(instance);

        SolutionConfig {
            add_config,
            mul_config,
            advice,
            instance,
        }
    }
}

impl<F: FieldExt> SolutionInstructions<F> for SolutionChip<F> {
    type Num = crate::Number<F>;

    fn load_private(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<F>,
    ) -> Result<<Self as SolutionInstructions<F>>::Num, Error> {
        let config = self.config();

        layouter.assign_region(
            || "load private",
            |mut region| {
                region
                    .assign_advice(|| "private input", config.advice, 0, || value)
                    .map(Number)
            },
        )
    }

    fn load_constants(
        &self,
        mut layouter: impl Layouter<F>,
    ) -> Result<[<Self as SolutionInstructions<F>>::Num; 3], Error> {
        let config = self.config();

        layouter.assign_region(
            || "load constants",
            |mut region| {
                let a = region
                    .assign_advice_from_instance(|| "a", config.instance, 0, config.advice, 0)
                    .map(Number)?;
                let b = region
                    .assign_advice_from_instance(|| "b", config.instance, 1, config.advice, 1)
                    .map(Number)?;
                let c = region
                    .assign_advice_from_instance(|| "c", config.instance, 2, config.advice, 2)
                    .map(Number)?;

                return Ok([a, b, c]);
            },
        )
    }

    fn expose_public(
        &self,
        mut layouter: impl Layouter<F>,
        num: <Self as SolutionInstructions<F>>::Num,
        row: usize,
    ) -> Result<(), Error> {
        let config = self.config();

        layouter.constrain_instance(num.0.cell(), config.instance, row)
    }

    fn solve_quadratic(
        &self,
        layouter: &mut impl Layouter<F>,
        a: <Self as SolutionInstructions<F>>::Num,
        b: <Self as SolutionInstructions<F>>::Num,
        _c: <Self as SolutionInstructions<F>>::Num,
        x: <Self as SolutionInstructions<F>>::Num,
    ) -> Result<<Self as SolutionInstructions<F>>::Num, Error> {
        let x2 = self.mul(layouter.namespace(|| "x2"), x.clone(), x.clone())?;
        let bx = self.mul(layouter.namespace(|| "bx"), b, x)?;
        let ax2 = self.mul(layouter.namespace(|| "ax2"), a, x2)?;
        self.add(layouter.namespace(|| "ax2 + bx"), ax2, bx)
    }
}

// The full circuit.
#[derive(Default)]
struct MyCircuit<F: FieldExt> {
    x: Value<F>,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = SolutionConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let advice = meta.advice_column();
        let instance = meta.instance_column();

        SolutionChip::configure(meta, advice, instance)
    }

    fn synthesize(
        &self,
        config: Self::Config,
        mut layouter: impl Layouter<F>,
    ) -> Result<(), Error> {
        let solution_chip = SolutionChip::<F>::construct(config);

        let x = solution_chip.load_private(layouter.namespace(|| "load x"), self.x)?;
        let [a, b, c] = solution_chip.load_constants(layouter.namespace(|| "load a,b,c"))?;

        let solution = solution_chip.solve_quadratic(&mut layouter, a, b, c, x)?;

        solution_chip.expose_public(layouter.namespace(|| "expose solution"), solution, 2)
    }
}

#[cfg(test)]
mod tests {
    use super::MyCircuit;
    use halo2_proofs::{circuit::Value, dev::MockProver, pasta::Fp};

    #[test]
    fn test_solve_quad() {
        let k = 5;
        let a = Fp::from(1);
        let b = Fp::from(2);
        let c = Fp::from(3);
        let x = Fp::from(1);

        let circuit = MyCircuit { x: Value::known(x) };

        let prover = MockProver::run(k, &circuit, vec![vec![a, b, c]]).unwrap();
        assert_eq!(prover.verify(), Ok(()));
    }

    #[test]
    fn test_fail_quad() {
        let k = 5;
        let a = Fp::from(1);
        let b = Fp::from(1);
        let c = Fp::from(3);
        let x = Fp::from(1);

        let circuit = MyCircuit { x: Value::known(x) };

        let prover = MockProver::run(k, &circuit, vec![vec![a, b, c]]).unwrap();
        assert!(prover.verify().is_err());
    }
}
