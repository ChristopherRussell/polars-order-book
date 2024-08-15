# Polars Order Book
Polars plugins which calculates aligned top of book information for order book data.

For example, calcualte the Best Bid and Best Offer prices at the time of each book update. Calculating the top N levels is also supported and is performant.

Input data can be in several forms.
- Each row contains a price level update: (side, price, updated_quantity)
- Each row contains an order mutation: (side, price, quantity_change)
- Each row contains an order mutation OR modify: (side, new_price, new_quantity_change, prev_price, prev_quantity_change)
Examples are given below.

Areas to be careful:
- If you are using order mutations but include summary messages.
    - E.g. some feeds may return a Trade 'Summary' message alongside normal Trade messages. 
- If you have inhomogenuous data from several different message types (adds, mods, deletes, trades) you will need to stack and sort them by sequence recieved.
- When converting order mutations to correct format.
    - Delete quantity needs to be set negative as a quantity diff is required.
    - For modifies, the quantity is the new and old values, not the change.
    - For adds, the quantity is correct.
    - For quantity-only modifies, you may need to fill the previous price column with the (un-modified) price.

Future work:
- Investigate parallelizing over side as a performance improvement and one-sided output as a feature.
    - Not doing this is fairly natural if we wish to align both the best bid and best offer with every row (regardless of the side of the book update).
    - However it may be of interest to some to only see the top of book for the book update's side, in which case this would be faster.
- Add functionallity for full order books (order level infromation, rather than price level).
    - The possiblities for what to track here is much greater and less well suited to tabular format as the number of orders can vary greatly per price level.
