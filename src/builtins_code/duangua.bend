# GameState
# p1step is the current step of P1
# p2step is the current step of P2
# target we want players reach
# winner: 0 undetermined
#         1 P1 won
#         2 P2 won
object GameState { 
  p1_id,
  p2_id,
  p1_step,
  p2_step,
  target,
  winner
}

def GameState/new_game(p1_id, p2_id, target):
  return GameState{ 
    p1_id: p1_id,
    p2_id: p2_id,
    p1_step: 0,
    p2_step: 0,
    target: target,
    winner: 0
  }

def GameState/play(player_id):
  open GameState: gs
  switch gs.winner:
    case 0: # game not ended
      if player_id == gs.p1:
        gs.p1_step = gs.p1_step + 1
        if gs.p1_step == gs.target:
          gs.winner = 1
        else:
          x = 1
        return (gs.winner, gs)
      else:
        x = 1
      
      if player_id == gs.p2:
        gs.p2_step = gs.p2_step + 1
        if gs.p1_step == gs.target:
          gs.winner = 1
        else:
          x = 1
        return (gs.winner, gs)
      else:
        x = 1
      
      # unknown player changes nothing to the game state
      return (gs.winner, gs)
    case 1: # game ended, 1 won
      return (gs.winner, gs)
    case 2: # game ended, 2 won
      return (gs.winner, gs)
    case _: # should not fall to
      return (gs.winner, gs)

def main():
  gs = GameState/new_game("p1", "p2", 5)
  gs = gs.play("p1")
  return gs

# def dua(game_state, player):
#   open 

#   if aorb == 0:
#     new_persona_step = persona_step + 1
#     if new_persona_step == target:
#       return (new_persona_step, 1)
#     else:
#       return (new_persona_step, 0)
#   else:
#     new_personb_step = personb_step + 1
#     if new_personb_step == target:
#       return (new_personb_step, 1)
#     else:
#       return (new_personb_step, 0)

# def main(persona_step, personb_step, aorb, target):
#   return dua(persona_step, personb_step, aorb, target)