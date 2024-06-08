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
      
  return PlayResult/Continues { 
      gs: GameState{ 
      steps: steps,
      target: target
    }
  }

def GameState/play(pr, player):
  match pr:
    case PlayResult/Won:
      return pr
    case PlayResult/Continues:
      gs = pr.gs
      open GameState: gs
      
      step = gs.steps[player] + 1 # get the current step for the player and increment it
      if step < gs.target:
        gs.steps[player] = step # update steps
        return PlayResult/Continues { gs: GameState { steps: gs.steps, target: gs.target } }
      else:
        return PlayResult/Won { player: player } # The game has ended, so we just return the winner

def main():
  pr = GameState/new_game(3, 3)
  pr = GameState/play(pr, 1) 
  pr = GameState/play(pr, 1) 
  pr = GameState/play(pr, 1)
  return pr