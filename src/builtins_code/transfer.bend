def transfer(from_amount, to_amount, transfer_amount):
  return (from_amount - transfer_amount, to_amount + transfer_amount)

def main():
  return transfer(1, 2, 3)