object GameState { 
  steps,
  target
}

type PlayResult:
  Won { player }
  Continues { gs }

def GameState/new_game(players, target):
  # Initialize `steps`, setting each `steps[i]` (which represents step of player i) to 0.
  bend i = 0, steps = Map/empty: # loop
    when i < players:
      steps[i] = 0 # Set the "step" of i'th player to 0
      steps = fork(i+1, steps) # Continue the loop, incrementing the i by 1 and passing current `steps`
      # to the next iteration
    else:
      steps = steps # Once done, just set "steps" to the calculated value.
      
  return GameState{ 
    steps: steps,
    target: target
  }

def GameState/play(gs, player):
  open GameState: gs

  step = gs.steps[player] + 1 # get the current step for the player and increment it
  if step < gs.target:
    # Not the winner yet
    gs.steps[player] = step # update steps
    # The game continues, so we return the new state
    return PlayResult/Continues { gs: GameState { steps: gs.steps, target: gs.target } }
  else:
    return PlayResult/Won { player: player } # The game has ended, so we just return the winner

def main(a,b,c):
  gs = GameState/new_game(3, 3) # create a game of two players with target of 1
#   match result = GameState/play(gs, 1):
#     case PlayResult/Won:
#       # Winner is saved to `result.player`
#       return "Won!"
#     case PlayResult/Continues:
#       # new game state is saved to `result.gs`
#       return "The game continues on..."
  return GameState/play(gs, 1) # Player 2 makes a step
#   return gs