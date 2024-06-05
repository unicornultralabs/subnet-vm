object Balance { val }

# bend run transfer2.bend "Balance { val: 1 }" "Balance { val: 2 }" "1"
# def main(from, to, amount):
#   open Balance: from 
#   open Balance: to
#   return (Balance { val: from.val - amount}, Balance { val: to.val + amount })
