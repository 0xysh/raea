//! The body's own measures (docs/hands.feature): joint limits and the hand prior, on the stand-in body.

use glam::{Quat, Vec3};
use raea_sys_body::prior::{coefficients, oddness, shape};
use raea_sys_body::*;

#[test]
fn limits_read_and_hold_a_joint() {
    let s = mannequin();
    // the rest pose is inside every range
    assert!(violations(&s, &s.rest(), 0.01).is_empty(), "{:?}", violations(&s, &s.rest(), 0.01));
    for side in Side::BOTH {
        let [upper, elbow, hand] = s.arm(side).unwrap();
        // an elbow's fold reads as flex alone, the same on both sides; bent back it is past its range, and is brought to it
        let hinge = s.offset(elbow).cross(s.offset(hand)).normalize();
        let a = angles_of(&s, elbow, Quat::from_axis_angle(hinge, 1.0)).unwrap();
        assert!((a.flex - 57.3).abs() < 1.0 && a.side.abs() < 1.0 && a.twist.abs() < 1.0, "{a:?}");
        let back = Quat::from_axis_angle(hinge, -1.2);
        let mut p = s.rest();
        p.turns[elbow] = back;
        assert_eq!(violations(&s, &p, 0.5)[0].0, elbow);
        let (held, by) = clamp_turn(&s, elbow, back);
        assert!((angles_of(&s, elbow, held).unwrap().flex + 50.0).abs() < 0.5 && by > 15.0);
        // a turn inside its range comes back as it is
        let fine = Quat::from_axis_angle(Vec3::Z, 0.3 * side.sign());
        assert_eq!(clamp_turn(&s, upper, fine), (fine, 0.0));
        // a turn reads the same whichever sign its numbers carry (it read the long way round, 329 degrees for 31, and
        // was then held to a limit it was nowhere near)
        let small = Quat::from_axis_angle(Vec3::new(0.3, 0.5, 0.8).normalize(), 0.5);
        assert!((clamp_turn(&s, hand, -small).1 - clamp_turn(&s, hand, small).1).abs() < 0.01 && clamp_turn(&s, hand, small).1 < 10.0);
        let (a, b) = (angles_of(&s, hand, small).unwrap(), angles_of(&s, hand, -small).unwrap());
        assert!((a.flex - b.flex).abs() < 0.01 && (a.side - b.side).abs() < 0.01 && (a.twist - b.twist).abs() < 0.01, "{a:?} {b:?}");
        // a wrist's twist past 25 degrees is held there
        let twisted = Quat::from_axis_angle((s.joints[hand].tail - s.joints[hand].head).normalize(), 0.9 * side.sign());
        let (held, _) = clamp_turn(&s, hand, twisted);
        assert!((angles_of(&s, hand, held).unwrap().twist - 25.0).abs() < 0.5);
    }
}

#[test]
fn the_hand_prior_shapes_and_reads_hands() {
    let s = mannequin();
    for side in Side::BOTH {
        let rest = hand_rest(&s, side).unwrap();
        let hand = s.arm(side).unwrap()[2];
        // how far each fingertip is from its knuckle, on average: longest with the hand open, shortest in a fist
        let span = |pose: &Pose| {
            let w = s.fk(pose);
            ["index", "middle", "ring", "pinky"].iter().map(|f| {
                let [knuckle, _, tip] = s.finger(f, side).unwrap();
                (w.pos[tip] + w.rot[tip] * (s.joints[tip].tail - s.joints[tip].head) - w.pos[knuckle]).length()
            }).sum::<f32>() / 4.0
        };
        let shaped = |z: &[f32]| {
            let mut p = s.rest();
            shape(&s, &mut p.turns, side, z).unwrap();
            p
        };
        let (mean, open, shut) = (shaped(&[]), shaped(&[2.0]), shaped(&[-2.0]));
        // the first component opens and closes the hand, the average hand between: open at +2, a fist at -2
        assert!(span(&open) > span(&mean) && span(&mean) > span(&shut) + 0.004, "{} {} {}", span(&open), span(&mean), span(&shut));
        // and the fingers close toward the palm, not away from it
        let w = s.fk(&shut);
        let [knuckle, _, tip] = s.finger("middle", side).unwrap();
        assert!((w.pos[tip] - w.pos[knuckle]).dot(w.rot[hand] * rest.palm) > 0.01);
        // a shape reads back as the numbers that made it, with nothing left over
        let z = [0.8, -0.5, 0.3, 1.1, -0.7];
        let (back, left) = coefficients(&s, &shaped(&z), side).unwrap();
        for (a, b) in z.iter().zip(&back) {
            assert!((a - b).abs() < 0.05, "{z:?} read back as {:?}", &back[..5]);
        }
        assert!(left < 0.5, "{left} degrees left over");
        // the average hand is the least odd; the rig's rest hand reads about 2 (it isn't SOMA's hand); fingers bent
        // back read over twice that
        assert!(oddness(&s, &mean, side).unwrap() < 0.05);
        assert!(oddness(&s, &s.rest(), side).unwrap() < 3.0);
        let mut broken = s.rest();
        for f in ["index", "middle", "ring", "pinky"] {
            let j = s.finger(f, side).unwrap()[1];
            let axis = (s.joints[j].tail - s.joints[j].head).normalize().cross(rest.palm).normalize();
            broken.turns[j] = Quat::from_axis_angle(axis, -1.2);
        }
        let (odd, usual) = (oddness(&s, &broken, side).unwrap(), oddness(&s, &s.rest(), side).unwrap());
        assert!(odd > 2.0 * usual, "fingers bent back read {odd}, the rest hand {usual}");
    }
}
