use geometry::*;
use actors::*;

#[inline]
fn interpolate_f32(before: f32, after: f32, alpha: f32) -> f32 {
    (1. - alpha) * before + alpha * after
}

#[inline]
fn interpolate_vec2(before: Vec2, after: Vec2, alpha: f32) -> Vec2 {
    Vec2{
        x: interpolate_f32(before.x, after.x, alpha),
        y: interpolate_f32(before.y, after.y, alpha),
    }
}

#[inline]
fn interpolate_trans(before: Transform, after: Transform, alpha: f32) -> Transform {
    Transform{
        pos: interpolate_vec2(before.pos, after.pos, alpha),
        rotation: interpolate_f32(before.rotation, after.rotation, alpha),
    }
}

#[inline]
fn interpolate_bullet(before: &Bullet, after: &Bullet, alpha: f32) -> Bullet {
    assert!(before.spec == after.spec);
    Bullet{
        spec: before.spec,
        trans: interpolate_trans(before.trans, after.trans, alpha),
        age: interpolate_f32(before.age, after.age, alpha),
    }
}

#[inline]
fn interpolate_camera(before: &Camera, after: &Camera, alpha: f32) -> Camera {
    Camera{
        pos: interpolate_vec2(before.pos, after.pos, alpha),
        vel: interpolate_vec2(before.vel, after.vel, alpha),
    }
}

#[inline]
fn interpolate_ship(before: &Ship, after: &Ship, alpha: f32) -> Ship {
    assert!(before.spec == after.spec);
    Ship{
        spec: before.spec,
        trans: interpolate_trans(before.trans, after.trans, alpha),
        vel: interpolate_vec2(before.vel, after.vel, alpha),
        camera: interpolate_camera(&before.camera, &after.camera, alpha),
        // TODO should we bump here?  and in extrapolate?        
        not_firing_for: before.not_firing_for,
        accel: before.accel,
        rotating: before.rotating,
    }
}

#[inline]
fn interpolate_shooter(before: &Shooter, after: &Shooter, _alpha: f32) -> Shooter {
    assert!(before.spec == after.spec);
    *before
}

#[inline]
fn interpolate_actor(before: &Actor, after: &Actor, alpha: f32) -> Actor {
    match (*before, *after) {
        (Actor::Ship(ref before_ship), Actor::Ship(ref after_ship)) =>
            Actor::Ship(interpolate_ship(before_ship, after_ship, alpha)),
        (Actor::Shooter(ref before_shooter), Actor::Shooter(ref after_shooter)) =>
            Actor::Shooter(interpolate_shooter(before_shooter, after_shooter, alpha)),
        (Actor::Bullet(ref before_bullet), Actor::Bullet(ref after_bullet)) =>
            Actor::Bullet(interpolate_bullet(before_bullet, after_bullet, alpha)),
        _ =>
            unreachable!(),
    }
}

#[inline]
fn interpolate_actors(before: &Actors, after: &Actors, alpha: f32) -> Actors {
    let mut actors = Actors::prepare_new(after);
    for (actor_id, after_actor) in after.iter() {
        match before.get(*actor_id) {
            None =>
                actors.insert(*actor_id, after_actor.clone()),
            Some(before_actor) =>
                actors.insert(*actor_id, interpolate_actor(before_actor, after_actor, alpha)),
        }
    };
    actors
}

#[inline]
pub fn interpolate_game(before: &Game, after: &Game, alpha: f32) -> Game {
    Game{
        actors: interpolate_actors(&before.actors, &after.actors, alpha),
        time: interpolate_f32(before.time, after.time, alpha),
    }
}
