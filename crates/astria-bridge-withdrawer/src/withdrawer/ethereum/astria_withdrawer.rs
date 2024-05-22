use ethers::contract::abigen;

abigen!(
    AstriaWithdrawer,
    "./ethereum/out/AstriaWithdrawer.sol/AstriaWithdrawer.json"
);
