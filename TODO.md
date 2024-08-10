# UI
- [ ] Add a window that displays info about the clicked family member
  - [x] Add the window with hard-coded data
  - [x] fetch data from the server and use it
  - [ ] Add other info about every member (didn't decide yet)
        could make this a dynamic `jsonb` field that can be listed in egui dynamically

- [x] Layout the tree in an "aesthetically pleasing" way using the Reingold Tilford Algorithm

# Backend
- [x] "get members" endpoint that responds with a recursive tree structure that has all members of the family
- [x] "add members" endpoint that receives a member adds it to the database (permissions: admin)
- [ ] "add members with diff" endpoint that receives the diff from the client and adds it to the database (permissions: anyone)
- [x] "edit members" endpoint that receives the fields to change from the client and edits it on the database  (permissions: admin)
- [x] Add database
  - [x] "members" table as a one-to-many relation with itself with a `father_id` field and a `mother_id` field
  - [x] "users" table for users that can view/manage the tree
  - [x] "sessions" table to manage sessions of users
