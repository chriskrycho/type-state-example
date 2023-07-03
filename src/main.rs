//! Demonstrate one way of combining type state and
//!
//! This little demo assumes:
//!
//! - A basic understanding of Rust's module system, traits, and types.
//! - The underlying idea of type state. If you are unfamiliar [this post][cb]
//!   is a great introduction.
//!
//! [cb]: http://cliffle.com/blog/rust-typestate/

// We declare this module below the `main` function, and import all of its `pub`
// items so that they are available to use in `main`.
use state::*;

/// The overall flow through `main` here demonstrates a three-step state machine
/// where state can flow from `First` to `Second` to `Third`, can step back from
/// `Third` to `Second`, and can exit from the state machine entirely once in
/// the `Third` state.
///
/// The key difference from normal state machine implementations using an enum
/// is that the *variants* are only part of the story: the *state structs* they
/// wrap are the other half. Each variant is a tuple-style variant which
/// includes the
fn main() {
    // We're just using this for the sake of "logging" some output. Feel free to
    // ignore it!
    let mut step = 0;

    // Here is the meat of the demo: we create a `StateMachine`

    // Because we have a not made the internal values of the state structs in
    // the `state` module public, this is actually the only way to set up the
    // state machine! This gives us a useful hook for passing initial state into
    // the state machine.
    let mut state_machine = StateMachine::new(1);

    loop {
        step += 1;
        println!("Step {step}: {state_machine:?}");

        // Now, because we have defined the `StateMachine` type so that each
        // variant wraps a "type state" (`First`, `Second`, and `Third`), we can
        // do normal pattern matching on its variants, but the transitions are
        // more or less predefined for us!
        match state_machine {
            // When we have an `A`, we can stay in the initial state, or we can
            // advance the machine to `Second` by using the public API provided
            // by `A` to get a `B`. Moreover, because `A` is *moved* into `B`,
            // once we are in `StateMachine::Second(B)`, we *cannot* go backward
            // using the public APIs we have defined.
            StateMachine::First(a) => {
                // Here, we model the "conditional behavior" with a random
                // choice between `true` and `false`, but in a real program,
                // this could be substantially more sophisticated!
                state_machine = if rand::random() {
                    StateMachine::First(a.add(rand::random()))
                } else {
                    StateMachine::Second(a.into_second(12))
                };
            }

            // When we have a `Second(B)`, we can again stay in that state *or*
            // move to `Third(C)`, but we again cannot go backwards.
            StateMachine::Second(b) => {
                state_machine = if rand::random() {
                    match b.into_third(4) {
                        Ok(third) => StateMachine::Third(third),
                        Err((original, reason)) => {
                            eprintln!("{reason}");
                            StateMachine::Second(original)
                        }
                    }
                } else {
                    StateMachine::Second(b.add(2))
                };
            }

            // But not being able to go backwards is a choice in terms of your
            // type state! If your types allow it, as they do here, then you can
            // go backward.
            StateMachine::Third(c) => {
                if rand::random() {
                    state_machine = StateMachine::Third(c.add(3.0));
                    break;
                }

                state_machine = StateMachine::Second(c.into_b(3));
            }
        }
    }

    println!("Total steps: {step}. Final state: {state_machine:?}");
}

/// The `state` module provides a privacy boundary, which is key to making the
/// pattern shown in the rest of this system work as expected.
mod state {
    /// The definition of the state machine itself is one part of the guarantees
    /// this pattern allows us to provide: each variant wraps a specific type,
    /// and *only* that type.
    ///
    /// Because the internals of the wrapped types are private to the module
    /// (see the discussion on the definitions of the types below!), they cannot
    /// be directly constructed. This in turn means that we can pattern match on
    /// the enum and get at the wrapped values, but to produce new states in the
    /// state machine, a caller can only (and must) use the API provided by each
    /// wrapped type to transition to another wrapped type.
    ///
    /// This leaves the caller in charge of *when* to transition between states,
    /// which is often important for state machines like this, but it puts this
    /// module squarely in charge of *how* to transition.
    ///
    /// There are more complicated versions of this you can implement, but all
    /// of them are compatible with this version.
    #[derive(Debug)]
    pub enum StateMachine {
        First(First),
        Second(Second),
        Third(Third),
    }

    impl StateMachine {
        pub fn new(initial: u32) -> Self {
            StateMachine::First(First(initial))
        }
    }

    // Next, we define a set of structs with distinct states. The states here
    // are not interesting in and of themselves: they are just numbers, so that
    // the state changes between them can be more or less trivial. The point is
    // that their internal state is *private* to them, so nothing outside the
    // `state` module has access to them. We have to be disciplined *within*
    // this module (though we could also enforce that safety by putting them in
    // their own modules if that was important for the structure of our code).

    ///
    #[derive(Debug)]
    pub struct First(u32);

    #[derive(Debug)]
    pub struct Second(i32);

    #[derive(Debug)]
    pub struct Third(f64);

    impl First {
        pub fn into_second(self, addend: i32) -> Second {
            Second(self.0 as i32 + addend)
        }

        pub fn add(&self, addend: u32) -> Self {
            First(self.0 + addend)
        }
    }

    impl Second {
        /// Get a `Third` from a `Second`.
        ///
        /// Similarly to `First::into_second`, we can include additional state
        /// required to accomplish the transformation. This is also a fallible
        /// operation (though we do not usually think about it!) since the
        /// multiplication could overflow. This demonstrates another handy
        /// result of splitting out the pattern matching from the internal state
        /// transitions: it allows the fallibility to be handled distinctly.
        ///
        /// One key bit here: since we are taking ownership of `self` here (as
        /// is typical of the type state pattern), it is important that we also
        /// return it when in a failure case, so that the state machine remains
        /// viable -- but this is specific to the state machine. It could also
        /// be the case that some attempts to transition states are
        /// unrecoverable failures, in which case you would *not* return `Self`
        /// and the type itself would encode that unrecoverability.
        pub fn into_third(self, factor: i32) -> Result<Third, (Self, String)> {
            self.0
                .checked_mul(factor)
                .ok_or_else(|| (self, "Overflow! 😱".into()))
                .map(f64::from)
                .map(Third)
        }

        pub fn add(&self, addend: i32) -> Self {
            Second(self.0 + addend)
        }
    }

    impl Third {
        pub fn add(&self, addend: f64) -> Self {
            Third(self.0 + addend)
        }

        pub fn into_b(self, divisor: i32) -> Second {
            Second(self.0 as i32 / divisor)
        }
    }
}
