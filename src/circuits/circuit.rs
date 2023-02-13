use plonkit::bellman_ce::{Circuit, ConstraintSystem, Engine, PrimeField, SynthesisError};

pub struct DemoCircuit<E: Engine> {
    pub root_hash: Option<E::Fr>,
}

impl<'a, E: Engine> Circuit<E> for DemoCircuit<E> {
    fn synthesize<CS: ConstraintSystem<E>>(self, cs: &mut CS) -> Result<(), SynthesisError> {
        Ok(())
    }
}

//
// // proving that I know x such that x^3 + x + 5 == 35
// // Generalized: x^3 + x + 5 == out
// pub struct CubeDemo<E: Engine> {
//     pub x: Option<E::Fr>,
// }
//
// impl<E: Engine> Circuit<E> for CubeDemo<E> {
//     fn synthesize<CS: ConstraintSystem<E>>(
//         self,
//         cs: &mut CS,
//     ) -> Result<(), SynthesisError>
//     {
//         // Flattened into quadratic equations (x^3 + x + 5 == 35):
//         // x * x = tmp_1
//         // tmp_1 * x = y
//         // y + x = tmp_2
//         // tmp_2 + 5 = out
//         // Resulting R1CS with w = [one, x, tmp_1, y, tmp_2, out]
//
//         // Allocate the first private "auxiliary" variable
//         let x_val = self.x;
//         let x = cs.alloc(|| "x", || {
//             x_val.ok_or(SynthesisError::AssignmentMissing)
//         })?;
//
//         // Allocate: x * x = tmp_1
//         let tmp_1_val = x_val.map(|mut e| {
//             e.square();
//             e
//         });
//         let tmp_1 = cs.alloc(|| "tmp_1", || {
//             tmp_1_val.ok_or(SynthesisError::AssignmentMissing)
//         })?;
//         // Enforce: x * x = tmp_1
//         cs.enforce(
//             || "tmp_1",
//             |lc| lc + x,
//             |lc| lc + x,
//             |lc| lc + tmp_1,
//         );
//
//         // Allocate: tmp_1 * x = y
//         let x_cubed_val = tmp_1_val.map(|mut e| {
//             e.mul_assign(&x_val.unwrap());
//             e
//         });
//         let x_cubed = cs.alloc(|| "x_cubed", || {
//             x_cubed_val.ok_or(SynthesisError::AssignmentMissing)
//         })?;
//         // Enforce: tmp_1 * x = y
//         cs.enforce(
//             || "x_cubed",
//             |lc| lc + tmp_1,
//             |lc| lc + x,
//             |lc| lc + x_cubed,
//         );
//
//         // Allocating the public "primary" output uses alloc_input
//         let out = cs.alloc_input(|| "out", || {
//             let mut tmp = x_cubed_val.unwrap();
//             tmp.add_assign(&x_val.unwrap());
//             tmp.add_assign(&E::Fr::from_str("5").unwrap());
//             Ok(tmp)
//         })?;
//         // tmp_2 + 5 = out
//         // => (tmp_2 + 5) * 1 = out
//         cs.enforce(
//             || "out",
//             |lc| lc + x_cubed + x + (E::Fr::from_str("5").unwrap(), CS::one()),
//             |lc| lc + CS::one(),
//             |lc| lc + out,
//         );
//         // lc is an inner product of all variables with some vector of coefficients
//         // bunch of variables added together with some coefficients
//
//         // usually if mult by 1 can do more efficiently
//         // x2 * x = out - x - 5
//
//         // mult quadratic constraints
//         //
//
//         Ok(())
//     }
// }