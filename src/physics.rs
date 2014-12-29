extern crate sdl2;

use geometry::Vec2;

#[deriving(PartialEq, Clone, Copy, Show)]
pub struct State {
    pub pos: Vec2,                    // Position
    pub v: Vec2,                      // Velocity
}


#[deriving(PartialEq, Clone, Copy)]
struct Derivative {
    dpos: Vec2,                   // dpos/dt = velocity
    dv: Vec2,                     // dv/dt = acceleration
}

pub trait Acceleration {
    // Gets acceleration given a certain state
    fn acceleration(&self, state: &State) -> Vec2;
}

#[inline]
fn evaluate<T: Acceleration>(x: &T, state: &State, dt: f64, d: Derivative) -> Derivative {
    let state = State{
        pos: state.pos + d.dpos*dt,
        v: state.v + d.dv*dt,
    };
    Derivative{dpos: state.v, dv: x.acceleration(&state)}
}

#[inline]
pub fn integrate<T: Acceleration>(x: &T, state: &State, dt: f64) -> State {
    let a = evaluate(x, state, 0., Derivative{dpos: Vec2::zero(), dv: Vec2::zero()});
    let b = evaluate(x, state, dt*0.5, a);
    let c = evaluate(x, state, dt*0.5, b);
    let d = evaluate(x, state, dt, c);
    let dposdt = (a.dpos + (b.dpos + c.dpos)*2. + d.dpos) * 1./6.;
    let dvdt = (a.dv + (b.dv + c.dv)*2. + d.dv) * 1./6.;
    State{
        pos: state.pos + dposdt * dt,
        v: state.v + dvdt * dt,
    }
}

pub trait Interpolate {
    fn interpolate(&self, next: &Self, alpha: f64) -> Self;
}

impl Interpolate for State {
    #[inline]
    fn interpolate(&self, current: &State, alpha: f64) -> State {
        let previous = self;
        State {
            pos: current.pos*alpha + previous.pos*(1.-alpha),
            v: current.v*alpha + previous.v*(1.-alpha),
        }
    }
}

// #[test]
// fn test_interpolate() {
//     let state_1 = State {
//         pos: Vec2{x: 3, y: 2},
//         v: Vec2{x: -1, y: 4},
//     }
//     let state_2 = State {
        
//     }
// }
