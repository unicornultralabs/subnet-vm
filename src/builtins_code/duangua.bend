type Win = True | False

def dua(persona_step, personb_step, aorb, target):
  if aorb == 0:
    new_persona_step = persona_step + 1
    if new_persona_step == target:
      return (new_persona_step, Win/True)
    else:
      return (new_persona_step, Win/False)
  else:
    new_personb_step = personb_step + 1
    if new_personb_step == target:
      return (new_personb_step, Win/True)
    else:
      return (new_personb_step, Win/False)

def main(persona_step, personb_step, aorb, target):
  return dua(persona_step, personb_step, aorb, target)