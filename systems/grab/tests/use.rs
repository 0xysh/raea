//! The measures docs/hands.feature names for what is done with a thing: passing it, moving it where it is, a
//! throw's wind-up, and keeping the arms, the knees and the thing out of the scene's solids.

use glam::{Quat, Vec3};
use raea_sys_body::*;
use raea_sys_grab::*;

fn palm_off(s: &Skeleton, f: &Frame, hold: &Hold) -> f32 {
    let (w, rest) = (s.fk(&f.pose), hand_rest(s, hold.side).unwrap());
    let hand = s.arm(hold.side).unwrap()[2];
    (w.pos[hand] + w.rot[hand] * rest.mid - f.thing.world(hold.at)).length()
}

#[test]
fn a_cup_passes_from_one_hand_to_the_other() {
    let s = mannequin();
    let cup = Thing::new(Solid::Cylinder { half: 0.05, r: 0.035 }, Vec3::new(-0.2, 1.0, 0.38));
    let Grip::One(right) = find_holds(&cup, s.rest().root, HoldOptions::default()) else { panic!() };
    let take = Take::new(cup, Grip::One(right), 0.0).passing(2.0);
    let left = right.across();
    assert_eq!(left.side, Side::L);
    let base = |_: f32| s.rest();
    let (mut t, mut last) = (0.0, perform(&s, &base, &take, 0.0).pose);
    while t < 4.0 {
        t += 1.0 / 30.0;
        let f = perform(&s, &base, &take, t);
        // somebody always has it: the right hand until the pass, the left from it, both at it
        if t >= take.grab && t <= 2.0 {
            assert!(palm_off(&s, &f, &right) < 0.015, "t {t}: the right palm is {} m off", palm_off(&s, &f, &right));
        }
        if t >= 2.0 {
            assert!(palm_off(&s, &f, &left) < 0.015, "t {t}: the left palm is {} m off", palm_off(&s, &f, &left));
        }
        assert!((f.thing.turn * Vec3::Y).dot(Vec3::Y) > 0.9999);
        // and no joint of the body jumps as the hands change (the arm let go swings back down in half a second, at up to 26 degrees a frame)
        let turned = f.pose.turns[..22].iter().zip(&last.turns).map(|(a, b)| a.angle_between(*b).to_degrees()).fold(0.0, f32::max);
        assert!(turned < 30.0, "t {t}: a joint turned {turned} degrees in a 30th of a second");
        last = f.pose;
    }
    // it ends on the left, and the right arm is back as the body has it
    let f = perform(&s, &base, &take, 3.9);
    assert!(f.thing.at.x > 0.1);
    let arm = s.arm(Side::R).unwrap();
    assert!(arm.iter().all(|&j| f.pose.turns[j].dot(s.rest().turns[j]).abs() > 0.9999));
}

#[test]
fn a_drawer_is_pulled_open_and_left_there() {
    let s = mannequin();
    // a drawer front at belly height, held by a handle on its near face, pulled 18 cm toward the body
    let front = Thing::new(Solid::Box { half: Vec3::new(0.2, 0.08, 0.01) }, Vec3::new(-0.1, 0.95, 0.55));
    let handle = Hold { side: Side::R, at: Vec3::new(0.0, 0.0, -0.01), normal: Vec3::NEG_Z, along: None, shape: Some([-0.8, 0.0, 0.0]) };
    let mut take = Take::new(front, Grip::One(handle), 0.0);
    let open = front.at - Vec3::Z * 0.18;
    take.slide = vec![(1.6, open, Quat::IDENTITY)];
    take.release = Some(Release::Leave { at: 1.9 });
    let base = |_: f32| s.rest();
    let mut t = take.grab;
    while t < 1.9 {
        let f = perform(&s, &base, &take, t);
        assert!(palm_off(&s, &f, &handle) < 0.03, "t {t}: the palm is {} m off the handle", palm_off(&s, &f, &handle));
        // it only slides: level, along its runners
        assert!((f.thing.at.x - front.at.x).abs() < 1e-5 && (f.thing.at.y - front.at.y).abs() < 1e-5);
        t += 1.0 / 30.0;
    }
    // left open where it was pulled to, the body back on its own
    let f = perform(&s, &base, &take, 3.0);
    assert!((f.thing.at - open).length() < 1e-5);
    assert_eq!(f.pose, s.rest());
}

#[test]
fn a_throw_is_wound_up_and_followed_through() {
    let s = mannequin();
    let ball = Thing::new(Solid::Ball { r: 0.05 }, Vec3::new(-0.2, 1.0, 0.38));
    let mut take = Take::new(ball, find_holds(&ball, s.rest().root, HoldOptions::default()), 0.0);
    let velocity = Vec3::new(0.0, 2.0, 4.0);
    take.release = Some(Release::Throw { at: 2.0, velocity, floor: 0.0 });
    let base = |_: f32| s.rest();
    let along = |t: f32| perform(&s, &base, &take, t).thing.at.dot(velocity.normalize());
    // drawn back against the throw before the release, then through where it was carried, moving forward
    let (carried, back, at) = (along(1.5), along(1.8), along(2.0));
    assert!(back < carried - 0.1, "drawn back {} m", carried - back);
    assert!((at - carried).abs() < 1e-3 && along(1.97) < at);
    // the hand goes out after it before coming back
    let hand = s.arm(Side::R).unwrap()[2];
    let reach = |t: f32| s.fk(&perform(&s, &base, &take, t).pose).pos[hand].dot(velocity.normalize());
    assert!(reach(2.12) > reach(2.0) + 0.03, "the hand follows through {} m", reach(2.12) - reach(2.0));
    assert_eq!(perform(&s, &base, &take, 2.6).pose, s.rest());
}

#[test]
fn the_forearms_the_knees_and_the_thing_keep_out_of_a_table() {
    let s = mannequin();
    let table = Thing::new(Solid::Box { half: Vec3::new(0.6, 0.475, 0.34) }, Vec3::new(0.0, 0.475, 0.72));
    // a box on the floor in front of the table, lifted onto it
    let thing = Thing::new(Solid::Box { half: Vec3::new(0.16, 0.1, 0.12) }, Vec3::new(0.0, 0.1, 0.2));
    let mut take = Take::new(thing, find_holds(&thing, s.rest().root, HoldOptions::default()), 0.0);
    take.avoid = vec![table];
    take.release = Some(Release::Put { at: 1.6, down: 2.3, place: Vec3::new(0.0, 1.05, 0.52) });
    let base = |_: f32| s.rest();
    let joints: Vec<usize> = ["calf_l", "calf_r", "lowerarm_l", "lowerarm_r"].iter().map(|n| s.joint(n).unwrap()).collect();
    let mut t = 0.0;
    while t < 2.3 {
        let f = perform(&s, &base, &take, t);
        let w = s.fk(&f.pose);
        for &j in &joints {
            assert!(table.distance(w.pos[j]) > 0.0, "t {t}: {} is in the table", s.joints[j].name);
        }
        // the box's corners clear the table on the way (it ends standing on it)
        if t > take.grab + 0.1 && t < 2.2 {
            let deep = f.thing.corners().iter().map(|c| -table.distance(*c)).fold(f32::MIN, f32::max);
            assert!(deep < 0.005, "t {t}: a corner of the box is {deep} m into the table");
        }
        t += 1.0 / 30.0;
    }
}

#[test]
fn a_body_turned_and_moved_takes_what_is_in_front_of_it() {
    let s = mannequin();
    // the body stands 2 m off, turned a third of the way round; the cup is where it was in front of it
    let (turn, at) = (Quat::from_rotation_y(2.1), Vec3::new(2.0, 0.0, -1.0));
    let mut stand = s.rest();
    stand.root = turn * stand.root + at;
    stand.turns[0] = turn * stand.turns[0];
    let base = move |_: f32| stand.clone();
    let cup = Thing { solid: Solid::Cylinder { half: 0.05, r: 0.035 }, at: turn * Vec3::new(-0.2, 1.0, 0.38) + at, turn };
    let (there, facing) = stand_for(&cup, base(0.0).root);
    // it already stands about where a take would have it stand, facing the cup
    assert!((there - base(0.0).root).length() < 0.25, "{}", (there - base(0.0).root).length());
    assert!(Quat::from_rotation_y(facing).angle_between(turn) < 0.6);
    let grip = find_holds(&cup, base(0.0).root, HoldOptions::default());
    let Grip::One(hold) = grip else { panic!("one hand") };
    let mut take = Take::new(cup, grip, 0.0);
    take.release = Some(Release::Put { at: 1.8, down: 2.4, place: turn * Vec3::new(0.05, 1.0, 0.4) + at });
    let mut t = take.grab;
    while t < 2.4 {
        let f = perform(&s, &base, &take, t);
        assert!(palm_off(&s, &f, &hold) < 0.012, "t {t}: the palm is {} m off", palm_off(&s, &f, &hold));
        // carried, it is in front of the turned body and upright
        assert!((f.thing.turn * Vec3::Y).dot(Vec3::Y) > 0.9999);
        t += 1.0 / 30.0;
    }
    let carried = perform(&s, &base, &take, 1.5).thing.at - base(0.0).root;
    assert!(carried.dot(turn * Vec3::Z) > 0.15, "in front of where it faces");
}

#[test]
fn an_elbow_stays_out_of_the_trunk_when_a_hand_reaches_across() {
    let s = mannequin();
    // a cup well to the body's left, taken with the right hand
    let cup = Thing::new(Solid::Cylinder { half: 0.05, r: 0.035 }, Vec3::new(0.22, 1.05, 0.3));
    let take = Take::new(cup, find_holds(&cup, s.rest().root, HoldOptions::default()), 0.0);
    let elbow = s.arm(Side::R).unwrap()[1];
    let mut t = 0.0;
    while t < 1.5 {
        let f = perform(&s, &|_| s.rest(), &take, t);
        let w = s.fk(&f.pose);
        let body = torso(&s, &w).unwrap();
        assert!(body.distance(w.pos[elbow]) > 0.0, "t {t}: the elbow is {} m into the trunk", -body.distance(w.pos[elbow]));
        t += 1.0 / 30.0;
    }
}
