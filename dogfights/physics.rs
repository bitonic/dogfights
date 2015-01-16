extern crate sdl2;

use geometry::*;

#[derive(PartialEq, Clone, Copy, Show)]
pub struct State {
    pub pos: Vec2,                    // Position
    pub vel: Vec2,                      // Velocity
}

#[derive(PartialEq, Clone, Copy)]
struct Derivative {
    dpos: Vec2,                   // dpos/dt = vel
    dvel: Vec2,                   // dv/dt = accel
}

pub trait Acceleration {
    // Gets accel given a certain state
    fn accel(&self, state: &State) -> Vec2;
}

#[inline]
fn evaluate<T: Acceleration>(x: &T, state: &State, dt: f32, d: Derivative) -> Derivative {
    let state = State{
        pos: state.pos + d.dpos*dt,
        vel: state.vel + d.dvel*dt,
    };
    Derivative{dpos: state.vel, dvel: x.accel(&state)}
}

#[inline]
pub fn integrate<T: Acceleration>(x: &T, state: &State, dt: f32) -> State {
    let a = evaluate(x, state, 0., Derivative{dpos: Vec2::zero(), dvel: Vec2::zero()});
    let b = evaluate(x, state, dt*0.5, a);
    let c = evaluate(x, state, dt*0.5, b);
    let d = evaluate(x, state, dt, c);
    let dposdt = (a.dpos + (b.dpos + c.dpos)*2. + d.dpos) * 1./6.;
    let dveldt = (a.dvel + (b.dvel + c.dvel)*2. + d.dvel) * 1./6.;
    State{
        pos: state.pos + dposdt * dt,
        vel: state.vel + dveldt * dt,
    }
}
