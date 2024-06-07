def main(persona_step, personb_step, aorb, target):
  if aorb == 0:
    new_persona_step = persona_step + 1
    if new_persona_step == target:
      return (new_persona_step, personb_step, 1)
    else:
      return (new_persona_step, personb_step, 0)
  else:
    new_personb_step = personb_step + 1
    if new_personb_step == target:
      return (persona_step, new_personb_step, 2)
    else:
      return (persona_step, new_personb_step, 0)

# def main(persona_step, personb_step, aorb, target):
#   return dua(persona_step, personb_step, aorb, target)