//! Unified 3D Waypoint Navigation Graph & deterministic A* Pathfinding for Station HELIOS-09.
//!
//! Every node and edge is verified against the station's static obstacle AABBs so rolling robots
//! never clip into walls, crates, holotables, server racks, or the reactor core.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::types::{AgentKind, AgentZone};

#[derive(Clone, Copy, Debug)]
pub struct NavNode {
    pub id: usize,
    pub pos: [f32; 3],
    pub zone: AgentZone,
    pub is_pipe_tunnel: bool,
}

const D1: f32 = -14.95; // Deck 1: Concourse & Reactor
pub const D2: f32 = -3.88; // Deck 2: Hangar, Atrium, Pipe Tunnel, Solar Bay
pub const D2U: f32 = 3.225; // Deck 2 Upper: Atrium U-Mezzanine Walkways
const D3: f32 = 12.05; // Deck 3: Ops & Command Spire
const PAD: f32 = -1.42; // External Landing Pads B & C

pub const NAV_NODES: &[NavNode] = &[
    // --- Main Hangar Bay (Deck 2, 0..=8) ---
    NavNode {
        id: 0,
        pos: [-7.35, D2, 22.0],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 1,
        pos: [-7.35, D2, 26.8],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 2,
        pos: [-7.35, D2, 31.4],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 3,
        pos: [7.35, D2, 22.0],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 4,
        pos: [7.35, D2, 26.8],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 5,
        pos: [7.35, D2, 31.4],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 6,
        pos: [0.0, D2, 14.5],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 7,
        pos: [-4.2, D2, 16.2],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 8,
        pos: [4.2, D2, 16.2],
        zone: AgentZone::HangarBay,
        is_pipe_tunnel: false,
    },
    // --- Central Core Atrium (Deck 2, 9..=15) ---
    NavNode {
        id: 9,
        pos: [0.0, D2, 8.6],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 10,
        pos: [0.0, D2, 3.6],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 11,
        pos: [-4.2, D2, 3.6],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 12,
        pos: [4.2, D2, 3.6],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 13,
        pos: [-4.8, D2, -6.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 14,
        pos: [4.8, D2, -6.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 15,
        pos: [0.0, D2, -11.5],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    // --- Axial Pipe Tunnel (Deck 2, Single-Lane Mutex, 16..=18) ---
    NavNode {
        id: 16,
        pos: [0.0, D2, -15.8],
        zone: AgentZone::AxialPipeTunnel,
        is_pipe_tunnel: true,
    },
    NavNode {
        id: 17,
        pos: [0.0, D2, -20.0],
        zone: AgentZone::AxialPipeTunnel,
        is_pipe_tunnel: true,
    },
    NavNode {
        id: 18,
        pos: [0.0, D2, -24.2],
        zone: AgentZone::AxialPipeTunnel,
        is_pipe_tunnel: true,
    },
    // --- Aft Solar Engineering Bay (Deck 2, 19..=23) ---
    NavNode {
        id: 19,
        pos: [0.0, D2, -27.5],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 20,
        pos: [0.0, D2, -33.6],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 21,
        pos: [-2.0, D2, -33.6],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 22,
        pos: [2.0, D2, -33.6],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 23,
        pos: [0.0, D2, -39.6],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    // --- Ops & Command Spire (Deck 3, 24..=30) ---
    NavNode {
        id: 24,
        pos: [-5.8, D3, 5.2],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 25,
        pos: [5.8, D3, 5.2],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 26,
        pos: [-6.0, D3, -0.8],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 27,
        pos: [6.0, D3, -0.8],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 28,
        pos: [0.0, D3, -2.6],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 29,
        pos: [-5.8, D3, -6.5],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 30,
        pos: [5.8, D3, -6.5],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    // --- Concourse & Reactor Core (Deck 1, 31..=40) ---
    NavNode {
        id: 31,
        pos: [-5.6, D1, 5.4],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 32,
        pos: [5.6, D1, 5.4],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 33,
        pos: [-5.6, D1, 0.0],
        zone: AgentZone::ReactorCatwalk,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 34,
        pos: [5.6, D1, 0.0],
        zone: AgentZone::ReactorCatwalk,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 35,
        pos: [-5.6, D1, -5.2],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 36,
        pos: [5.6, D1, -5.2],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 37,
        pos: [-14.2, D1, 5.4],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 38,
        pos: [14.2, D1, 5.4],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 39,
        pos: [-14.2, D1, -5.2],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 40,
        pos: [14.2, D1, -5.2],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    // --- External Landing Pads B & C (41..=44) ---
    NavNode {
        id: 41,
        pos: [-31.3, PAD, -5.2],
        zone: AgentZone::PadBPort,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 42,
        pos: [-31.3, PAD, 3.5],
        zone: AgentZone::PadBPort,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 43,
        pos: [31.3, PAD, -5.2],
        zone: AgentZone::PadCStarboard,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 44,
        pos: [31.3, PAD, 3.5],
        zone: AgentZone::PadCStarboard,
        is_pipe_tunnel: false,
    },
    // --- Bespoke Room Activity Workstation Spurs (45..=53) ---
    // 45: Atrium Mini-Elevator Lower Approach (Deck 2 forward half, clear of pillar [6.8, 5.2])
    NavNode {
        id: 45,
        pos: [6.2, D2, 8.4],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    // 46..=47: Ops Holotable Upload Perimeter Spurs (Deck 3, R = 3.6 >= 2.9 clear of holotable)
    NavNode {
        id: 46,
        pos: [-3.6, D3, 1.0],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 47,
        pos: [3.6, D3, 1.0],
        zone: AgentZone::OpsDeck,
        is_pipe_tunnel: false,
    },
    // 48..=49: Deck 1 Port Spent-Rod Cooling Vat & Starboard Fresh Cryo-Cell Rack Docks
    NavNode {
        id: 48,
        pos: [-13.4, D1, 0.0],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 49,
        pos: [13.4, D1, 0.0],
        zone: AgentZone::Concourse,
        is_pipe_tunnel: false,
    },
    // 50..=53: Solar Bay Capacitor Tower Calibration Spurs (Deck 2, central X=0 lane stays clear)
    NavNode {
        id: 50,
        pos: [-2.3, D2, -29.8],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 51,
        pos: [2.3, D2, -29.8],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 52,
        pos: [-2.3, D2, -37.2],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 53,
        pos: [2.3, D2, -37.2],
        zone: AgentZone::SolarBay,
        is_pipe_tunnel: false,
    },
    // --- Atrium Ground Forward Control-Panel Workstations (54..=58, strictly Z >= +3.5) ---
    NavNode {
        id: 54,
        pos: [9.4, D2, 3.5],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 55,
        pos: [9.4, D2, 6.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 56,
        pos: [4.2, D2, 5.2],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 57,
        pos: [-4.2, D2, 5.2],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 58,
        pos: [-9.4, D2, 3.5],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    // --- Atrium Upper U-Mezzanine Walkways & Control-Panel Workstations (59..=68, Y = D2U) ---
    NavNode {
        id: 59,
        pos: [8.8, D2U, 6.2],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 60,
        pos: [8.8, D2U, 2.55],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 61,
        pos: [8.8, D2U, -2.55],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 62,
        pos: [8.8, D2U, -8.2],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 63,
        pos: [8.8, D2U, -12.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 64,
        pos: [3.6, D2U, -12.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 65,
        pos: [-3.6, D2U, -12.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 66,
        pos: [-8.8, D2U, -12.0],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 67,
        pos: [-8.8, D2U, -8.2],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
    NavNode {
        id: 68,
        pos: [-8.8, D2U, -2.55],
        zone: AgentZone::CentralAtrium,
        is_pipe_tunnel: false,
    },
];

/// Undirected obstacle-free navigation edges.
pub const NAV_EDGES: &[(usize, usize)] = &[
    // Hangar Bay lanes & airlock connectors
    (0, 1),
    (1, 2),
    (3, 4),
    (4, 5),
    (0, 7),
    (7, 6),
    (6, 8),
    (8, 3),
    // Hangar <-> Central Atrium (through Blast Door 0 at Z = 11.6)
    (6, 9),
    // Central Core Atrium loop around Elevator Shaft
    (9, 10),
    (10, 11),
    (10, 12),
    (11, 13),
    (12, 14),
    (13, 15),
    (14, 15),
    // Central Atrium <-> Axial Pipe Tunnel <-> Solar Engineering Bay (through Doors 1 & 2)
    (15, 16),
    (16, 17),
    (17, 18),
    (18, 19),
    // Solar Engineering Bay internal 2-lane diamond loop & capacitor inspection spurs
    (19, 20),
    (19, 21),
    (19, 22),
    (20, 21),
    (20, 22),
    (20, 23),
    (21, 23),
    (22, 23),
    // Ops Command Spire (Deck 3) ring around Central Holotable
    (24, 25),
    (24, 26),
    (25, 27),
    (26, 28),
    (27, 28),
    (26, 29),
    (27, 30),
    // Concourse & Reactor Core (Deck 1) loop around Reactor Core & Concourse wings
    (31, 32),
    (31, 33),
    (32, 34),
    (33, 35),
    (34, 36),
    (31, 37),
    (32, 38),
    (35, 39),
    (36, 40),
    (37, 39),
    (38, 40),
    // External Landing Pads B & C
    (41, 42),
    (43, 44),
    // Bespoke Room Activity Workstation Spurs
    // Atrium Forward Ground Control Panels & Mini-Elevator Approach (strictly clear of main elevator)
    (9, 45),
    (12, 54),
    (54, 55),
    (12, 56),
    (56, 9),
    (11, 57),
    (57, 9),
    (11, 58),
    // Atrium Upper U-Mezzanine Walkways (Starboard -> Rear Skybridge -> Port)
    (59, 60),
    (60, 61),
    (61, 62),
    (62, 63),
    (63, 64),
    (64, 65),
    (65, 66),
    (66, 67),
    (67, 68),
    // Ops Holotable Upload Spurs
    (24, 46),
    (26, 46),
    (25, 47),
    (27, 47),
    // Reactor Cooling Vat & Fresh Cryo-Cell Rack Docks
    (37, 48),
    (39, 48),
    (38, 49),
    (40, 49),
    // Solar Bay Capacitor Tower Calibration Spurs
    (19, 50),
    (21, 50),
    (19, 51),
    (22, 51),
    (21, 52),
    (23, 52),
    (22, 53),
    (23, 53),
];

#[inline]
pub fn dist3(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    (dx * dx + dy * dy + dz * dz).sqrt()
}

pub fn neighbors(node_id: usize) -> Vec<usize> {
    let mut out = Vec::new();
    for &(a, b) in NAV_EDGES {
        if a == node_id {
            out.push(b);
        } else if b == node_id {
            out.push(a);
        }
    }
    out
}

#[derive(Clone, Copy)]
struct AStarState {
    cost: f32,
    node: usize,
}

impl PartialEq for AStarState {
    fn eq(&self, other: &Self) -> bool {
        self.node == other.node
    }
}
impl Eq for AStarState {}
impl Ord for AStarState {
    fn cmp(&self, other: &Self) -> Ordering {
        other
            .cost
            .partial_cmp(&self.cost)
            .unwrap_or(Ordering::Equal)
            .then_with(|| other.node.cmp(&self.node))
    }
}
impl PartialOrd for AStarState {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// Deterministic A* shortest path on `NAV_NODES`, returning node IDs from the next step through `goal`.
pub fn find_path(start: usize, goal: usize) -> Vec<usize> {
    let empty = [false; 128];
    find_path_avoiding(start, goal, &empty[..NAV_NODES.len()])
}

/// Occupancy-aware A* pathfinding: penalizes nodes currently occupied by other robots so bots
/// naturally route around the opposite side of rooms instead of driving head-on into congestion.
pub fn find_path_avoiding(start: usize, goal: usize, occupied: &[bool]) -> Vec<usize> {
    let n = NAV_NODES.len();
    if start >= n || goal >= n || start == goal {
        return Vec::new();
    }

    let mut g_score = vec![f32::INFINITY; n];
    let mut came_from = vec![usize::MAX; n];
    let mut open = BinaryHeap::new();

    g_score[start] = 0.0;
    open.push(AStarState {
        cost: dist3(NAV_NODES[start].pos, NAV_NODES[goal].pos),
        node: start,
    });

    while let Some(AStarState { node: curr, .. }) = open.pop() {
        if curr == goal {
            let mut path = Vec::new();
            let mut cur = goal;
            while cur != start && cur != usize::MAX {
                path.push(cur);
                cur = came_from[cur];
            }
            path.reverse();
            return path;
        }
        for nb in neighbors(curr) {
            let occ_penalty = if nb != goal && occupied.get(nb).copied().unwrap_or(false) {
                24.0
            } else {
                0.0
            };
            let tentative =
                g_score[curr] + dist3(NAV_NODES[curr].pos, NAV_NODES[nb].pos) + occ_penalty;
            if tentative < g_score[nb] {
                came_from[nb] = curr;
                g_score[nb] = tentative;
                let f = tentative + dist3(NAV_NODES[nb].pos, NAV_NODES[goal].pos);
                open.push(AStarState { cost: f, node: nb });
            }
        }
    }

    Vec::new()
}

/// Curated goal node pool for each robot class & operating zone.
pub fn goal_pool_for_agent(
    kind: AgentKind,
    zone: AgentZone,
    cur_node: usize,
    carrying_cargo: bool,
) -> &'static [usize] {
    match (kind, zone) {
        (AgentKind::CargoForklift, AgentZone::HangarBay) => {
            if cur_node <= 2 {
                &[0, 1, 2]
            } else {
                &[3, 4, 5]
            }
        }
        (AgentKind::CargoForklift, AgentZone::PadBPort) => &[41, 42],
        (AgentKind::CargoForklift, AgentZone::PadCStarboard) => &[43, 44],
        (AgentKind::FloorScrubber, AgentZone::HangarBay) => &[6, 7, 8, 9, 10],
        (AgentKind::FloorScrubber, _) => &[9, 10, 11, 12],
        (AgentKind::PanelTechBot, _) => {
            if cur_node >= 59 {
                &[59, 60, 61, 62, 63, 64, 65, 66, 67, 68]
            } else {
                &[9, 10, 11, 12, 45, 54, 55, 56, 57, 58]
            }
        }
        (AgentKind::OpsHoloDroid, _) => &[24, 25, 26, 27, 28, 29, 30],
        (AgentKind::SecurityEnforcer, AgentZone::CentralAtrium) => &[9, 11, 13, 15, 14, 12],
        (AgentKind::SecurityEnforcer, _) => &[31, 37, 39, 35, 36, 40, 38, 32],
        (AgentKind::ReactorCoolantBot, _) => &[31, 33, 35, 36, 34, 32],
        (AgentKind::SolarCapacitorBot, _) => &[19, 20, 21, 22, 23],
        // Cross-room Courier Runner alternates Hangar/Atrium pickup <-> Solar Bay delivery!
        (AgentKind::CourierRunner, _) => {
            if carrying_cargo {
                &[19, 21, 22, 23]
            } else {
                &[7, 8, 11, 12]
            }
        }
        _ => &[10],
    }
}
