# GameState
# p1step is the current step of P1
# p2step is the current step of P2
# target we want players reach
# winner: 0 undetermined
#         1 P1 won
#         2 P2 won
object GameState { 
  p1_step,
  p2_step,
  target,
  winner
}

def GameState/new_game(target):
  return GameState{ 
    p1_step: 0,
    p2_step: 0,
    target: target,
    winner: 0
  }

def GameState/play(gs, player):
  open GameState: gs
  switch gs.winner:
    case 0: # game not ended
      switch player:
        case 0:
          # unknown player changes nothing to the game state
          return (gs, gs.winner)
        case 1:
          if gs.p1_step + 1 == gs.target:
            return GameState{ 
              p1_step: gs.p1_step + 1,
              p2_step: gs.p2_step,
              target: gs.target,
              winner: 1
            }
          else:
            return GameState{ 
              p1_step: gs.p1_step,
              p2_step: gs.p2_step + 1,
              target: gs.target,
              winner: 0
            }
        case 2:
          if gs.p2_step + 1 == gs.target:
            return GameState{ 
              p1_step: gs.p1_step,
              p2_step: gs.p2_step + 1,
              target: gs.target,
              winner: 2
            }
          else:
            return GameState{ 
              p1_step: gs.p1_step,
              p2_step: gs.p2_step + 1,
              target: gs.target,
              winner: 0
            }
        case _:
          # unknown player changes nothing to the game state
          return (gs, gs.winner)
    case 1: # game ended, 1 won
      return (gs, gs.winner)
    case 2: # game ended, 2 won
      return (gs, gs.winner)
    case _: # should not fall to
      return (gs, gs.winner)

# due to limitations of HVM, we cannot read data from this code
# however, we can feed the data from arguments
# so arguments of main is the thing we want to modify PLUS the actual arguments.
def main():
  gs = GameState/new_game(2)
  # gs = GameState/play(gs, 1)
  gs = GameState/play(gs, 0)
  return gs
