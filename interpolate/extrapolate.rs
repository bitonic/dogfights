// use geometry::*;
// use actors::*;
// use specs::*;
// use input::*;

// #[inline]
// fn extrapolate_pos(before: Vec2, vel: Vec2, dt: f32) -> Vec2 {
//     before + vel * dt
// }

// #[inline]
// fn extrapolate_rotation(before: f32, vel: f32, rotating: Rotating, dt: f32) -> f32 {
//     match rotating {
//         Rotating::Still => before,
//         Rotating::Left => before + vel * dt,
//         Rotating::Right => before - vel * dt,
//     }
// }

// #[inline]
// fn extrapolate_bullet(specs: &GameSpec, before: &Bullet, dt: f32) -> Bullet {
//     let spec = specs.get_spec(before.spec).is_bullet();
//     let vel = before.trans.pos.norm() * spec.vel;
//     Bullet{
//         spec: before.spec,
//         trans: Transform{
//             pos: extrapolate_pos(before.trans.pos, vel, dt),
//             rotation: before.trans.rotation,
//         },
//         age: before.age + dt,
//     }
// }

// #[inline]
// fn extrapolate_camera(before: &Camera, dt: f32) -> Camera {
//     Camera{
//         pos: extrapolate_pos(before.pos, before.vel, dt),
//         vel: before.vel,
//     }
// }

// #[inline]
// fn extrapolate_ship(specs: &GameSpec, before: &Ship, dt: f32) -> Ship {
//     let spec = specs.get_spec(before.spec).is_ship();
//     Ship{
//         spec: before.spec,
//         trans: Transform{
//             pos: extrapolate_pos(before.trans.pos, before.vel, dt),
//             rotation: extrapolate_rotation(before.trans.rotation, spec.rotation_vel, before.rotating, dt),
//         },
//         vel: before.vel,
//         // TODO should we bump here?  and in interpolate?
//         not_firing_for: before.not_firing_for,
//         accel: before.accel,
//         rotating: before.rotating,
//         camera: extrapolate_camera(&before.camera, dt),
//     }
// }

// #[inline]
// fn extrapolate_shooter(before: &Shooter, _dt: f32) -> Shooter {
//     *before
// }

// #[inline]
// fn extrapolate_actor(before: &Actor, dt: f32) -> 
