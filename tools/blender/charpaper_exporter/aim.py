import math

from mathutils import Quaternion
from mathutils import Vector

# Where the character looks and who turns how far, as the app works it out: a port of the app's
# charpaper-gaze crate, kept the same line for line so the preview shows what the app will.
# Angles are (yaw, pitch) in radians: yaw towards the character's left, pitch upwards.

# The eyes are counted on for this much of their reach; the head makes up the rest.
EYE_COMFORT = 0.9
# Eyes that are no eyeballs, or are moved by shape keys alone, are at their fullest for a
# target this far aside.
DRAWN_REACH = Vector((math.radians(35.0), math.radians(25.0)))
# Eyes this close to straight above the pivot, as a share of their distance from it, don't
# say which way the character faces.
LEAST_AHEAD = 0.05


def _clamp(value, low, high):
    return Vector((min(max(value.x, low.x), high.x), min(max(value.y, low.y), high.y)))


class Frame:
    """The character's own left, up and front in the rest pose. Angles are measured against it."""

    def __init__(self, front, up):
        self.up = up.normalized()
        self.front = (front - self.up * front.dot(self.up)).normalized()
        self.side = self.up.cross(self.front)

    def angles(self, direction):
        d = direction.normalized()
        yaw = math.atan2(d.dot(self.side), d.dot(self.front))
        return Vector((yaw, math.asin(min(max(d.dot(self.up), -1.0), 1.0))))

    def toward(self, angles):
        """The turn that points front along the angles, without rolling."""
        return Quaternion(self.up, angles.x) @ Quaternion(self.side, -angles.y)

    def direction(self, angles):
        return self.toward(angles) @ self.front

    def turn(self, start, end):
        """The turn that takes what points along `start` to point along `end`."""
        return self.toward(end) @ self.toward(start).inverted()


def front_from_eyes(pivot, eyes, up):
    """Which way the character faces, from where the eyes sit in the rest pose: level, square
    to the line between the eyes, on the side of the pivot, the head's joint, that they are on. None
    when the bones don't say."""
    if not eyes:
        return None

    def level(v):
        return v - up * v.dot(up)

    middle = sum(eyes, Vector()) / len(eyes)
    least = (middle - pivot).length * LEAST_AHEAD
    ahead = level(middle - pivot)
    square = up.cross(level(eyes[-1] - eyes[0]))
    if square.length > 1e-6:
        square.normalize()
        along = ahead.dot(square)
        return square * math.copysign(1.0, along) if abs(along) > least else None
    return ahead.normalized() if ahead.length > least else None


class Eyes:
    def __init__(self, limit, reach):
        self.limit = limit
        self.reach = reach

    def look(self, target):
        """The look for a target at these angles from the sockets: -1 to 1 each way."""
        look = Vector((target.x / max(self.reach.x, 1e-3), target.y / max(self.reach.y, 1e-3)))
        return _clamp(look, Vector((-1.0, -1.0)), Vector((1.0, 1.0)))

    def turn(self, look):
        return Vector((look.x * self.limit.x, look.y * self.limit.y))


class Setup:
    def __init__(self, turn_limit, neck_limit, head_share, head_turn, eyes):
        self.turn_limit = turn_limit
        self.neck_limit = neck_limit
        self.head_share = head_share
        self.head_turn = head_turn
        self.eyes = eyes

    def reach(self):
        """How far aside the character can look at all, with everything at its limit."""
        return self.turn_limit + (self.eyes.reach if self.eyes else Vector((0.0, 0.0)))

    def head(self, target, clip):
        """Where the head points for a target at these angles from the chest, when the pose
        it is in points it at `clip`."""
        share = min(max(self.head_turn, 0.0), 1.0) if self.eyes else 1.0
        reach = self.eyes.reach * EYE_COMFORT if self.eyes else Vector((0.0, 0.0))
        head = clip + (target - clip) * share
        left = target - head
        head = head + left - _clamp(left, -reach, reach)
        low = Vector((min(-self.turn_limit.x, clip.x), min(-self.turn_limit.y, clip.y)))
        high = Vector((max(self.turn_limit.x, clip.x), max(self.turn_limit.y, clip.y)))
        return _clamp(head, low, high)

    def neck(self, clip, head):
        """Where the neck's part of the turn alone leaves the head pointing."""
        part = (head - clip) * (1.0 - min(max(self.head_share, 0.0), 1.0))
        return clip + _clamp(part, -self.neck_limit, self.neck_limit)


def turn_from_rest(now, rest):
    """How far a bone has turned from its rest pose. Both in the armature's space."""
    return now @ rest.inverted()


def carried(turned, turn):
    """A turn worked out as a bone stood in its rest pose, as the armature sees it now that
    the bone has turned by `turned` since."""
    return turned @ turn @ turned.inverted()
