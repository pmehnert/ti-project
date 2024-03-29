pub mod lcp_array;
pub mod num;
pub mod sais;
pub mod suffix_array;

pub mod prelude {
    pub use crate::index::{ArrayIndex, ToIndex};
    pub use crate::lcp_array as lcp;
    pub use crate::num::*;
    pub use crate::suffix_array as sa;
}

use std::process::{ExitCode, Termination};
use std::time::{Duration, Instant};
use std::{env, fs, hint, io};

use crate::cast::Transmutable;
use crate::prelude::*;

pub fn main() -> Result<TestResults, String> {
    fn run_timed<T>(f: impl FnOnce() -> T) -> (T, Duration) {
        let before = Instant::now();
        let result = hint::black_box(f());
        let elapsed = before.elapsed();
        (result, elapsed)
    }

    fn try_run_sa<Idx>(text: &[u8]) -> Option<TestResults>
    where
        Idx: ArrayIndex + IntTypes + Transmutable<Idx::Signed>,
        Idx::Signed: ArrayIndex + Signed,
    {
        let (result, sa_time) = run_timed(|| sa::sais::<Idx>(text));
        let (sa_memory, sa) = result.ok()?;

        #[cfg(feature = "verify")]
        sa.verify(text);

        Some(TestResults { sa_time, sa_memory, ..run_lcp(sa) })
    }

    fn run_lcp<Idx: ArrayIndex>(sa: sa::SuffixArray<u8, Idx>) -> TestResults {
        let (_lcp_naive, lcp_naive_time) = run_timed(|| lcp::naive(&sa));
        let (_lcp_kasai, lcp_kasai_time) = run_timed(|| lcp::kasai(&sa.inverse()));
        let (_lcp_phi, lcp_phi_time) = run_timed(|| lcp::phi(&sa));

        #[cfg(feature = "verify")]
        {
            _lcp_naive.verify();
            _lcp_kasai.verify();
            _lcp_phi.verify();
        }

        TestResults { lcp_naive_time, lcp_kasai_time, lcp_phi_time, ..Default::default() }
    }

    fn run(text: &[u8]) -> Result<TestResults, String> {
        None.or_else(|| try_run_sa::<u32>(text))
            .or_else(|| try_run_sa::<u64>(text))
            .or_else(|| try_run_sa::<usize>(text))
            .ok_or_else(|| {
                format!("cannot find index type for text of length {}", text.len())
            })
    }

    let param = env::args().nth(1);
    let input_path = param.ok_or_else(|| "expected exactly 1 argument".to_owned())?;
    let input_file = fs::read(input_path).map_err(|e| e.to_string())?;
    run(&input_file)
}

#[derive(Debug, Clone, Copy, Default)]
pub struct TestResults {
    sa_time: Duration,
    sa_memory: usize,
    lcp_naive_time: Duration,
    lcp_kasai_time: Duration,
    lcp_phi_time: Duration,
}

impl Termination for TestResults {
    fn report(self) -> ExitCode {
        use io::Write;
        let _ = writeln!(
            io::stderr(),
            "RESULT name=Pascal\tMehnert \
            sa_construction_time={} \
            sa_construction_memory={} \
            lcp_naive_construction_time={} \
            lcp_kasai_construction_time={} \
            lcp_phi_construction_time={}",
            self.sa_time.as_millis(),
            self.sa_memory / (1 << 20),
            self.lcp_naive_time.as_millis(),
            self.lcp_kasai_time.as_millis(),
            self.lcp_phi_time.as_millis()
        );
        ExitCode::SUCCESS
    }
}

pub mod index {
    use crate::num::*;

    /// A trait for types that can be used to index texts.
    pub trait ArrayIndex: PrimInt + AsPrimitive<usize> {
        /// Convert a `usize` to `Self` with a primitve cast.
        fn from_usize(value: usize) -> Self;
    }

    impl<T: PrimInt + AsPrimitive<usize>> ArrayIndex for T
    where
        usize: AsPrimitive<T>,
    {
        #[inline(always)]
        fn from_usize(value: usize) -> Self { value.as_() }
    }

    /// An extensions trait to convert `usize`s to [`ArrayIndex`]s.
    pub trait ToIndex<Idx: ArrayIndex> {
        /// Convert `self` to a value of type `Idx` using a primitive cast.
        fn to_index(self) -> Idx;
    }

    impl<Idx: ArrayIndex> ToIndex<Idx> for usize {
        #[inline(always)]
        fn to_index(self) -> Idx { Idx::from_usize(self) }
    }
}

pub mod cast {
    /// A trait to allow for save casting between certain types.
    ///
    /// If type `A`  implements `Transmutable<B>`, then it is safe to cast
    /// between pointers of the two types. Additionally, it must be possible
    /// to cast between slices of type `A` and `B`. The property must be
    /// commutative, i.e. `B` must also be transmutable to `A`.
    ///
    /// In this project, the trait is used to convert between slices of signed
    /// signed unsigned integers of equal size, e.g. &[u32] as &[i32].
    ///
    /// # Safety
    ///
    /// Incorrect implementations of this trait may lead to undefined behaviour.
    /// Therefore the trait, and implementations thereof are marked as unsafe.
    pub unsafe trait Transmutable<T>: Sized {}

    #[doc(hidden)]
    macro_rules! impl_transmutable {
            ($( $left:ty =>  $right:ty ),*) => {
                $( unsafe impl Transmutable<$right> for $left {} )*
            };
        }

    unsafe impl<T> Transmutable<T> for T {}
    impl_transmutable!(u8 => i8, u16 => i16, u32 => i32, u64 => i64, usize => isize);
    impl_transmutable!(i8 => u8, i16 => u16, i32 => u32, i64 => u64, isize => usize);
}
