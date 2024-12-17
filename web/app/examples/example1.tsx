export const Example1 = `

shared {
  let Done = false;
  let Leader = 0;
}

program A(my_id: int) {

  let leader_id = my_id;

  send out(my_id);

  loop atomic wait receive in (x) => {
    print("receive", x);
      if x > leader_id {
        leader_id = x;
        send out(x);
      } else {
        if x == leader_id {
          print("finished");
          send out(x);
          break;
        }
      }
  };
  
  if my_id == leader_id {
    print("I AM THE LEADER!!!");
    ! {
        Done = true;
        Leader += 1;
    }
  }
}

always {
    !Done || (Leader == 1);
}

main {
  let a = run A(1);
  let b = run A(2);

  channel a.out (int)> b.in;
  channel b.out (int)> a.in;

  print("DONE");
}

`;
