type Win = True | False

def dua(current_step, target):
  new_step = current_step + 1
  if new_step == target:
    return (new_step, Win/True)
  else:
    return (new_step, Win/False)

def main:
  return dua()