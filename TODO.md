# UI
- [ ] Add a window that displays info about the clicked family member
  - [x] Add the window with hard-coded data
  - [x] fetch data from the server and use it
  - [ ] Add other info about every member (didn't decide yet)
- [x] Layout the tree in an "aesthetically pleasing" way using the Reingold Tilford Algorithm

# Backend
- [x] "get members" endpoint that responds with a recursive tree structure that has all members of the family
- [ ] "add members" endpoint that receives the diff from the client and adds it to the database
- [ ] "edit members" endpoint that receives the diff from the client and edits it on the database
- [ ] Add database
  - [x] "members" table as a one-to-many relation with itself with a `father_id` field and a `mother_id` field
