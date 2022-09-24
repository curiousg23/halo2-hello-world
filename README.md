# halo2-hello-world

Simple circuit in halo2.

Takes in three public inputs, $a, b, c$ and a private input $x$, and proves that $ax^2 + bx = c$. The circuit uses one instance and one advice column.

The circuit uses an `Add` chip and a `Mul` chip, which perform addition and multiplication respectively, on two private inputs. The chips are written to use one advice column.

## Usage

To run tests:

```
$ cargo test
```
