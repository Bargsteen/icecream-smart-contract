use concordium_std::*;

#[contract_state(contract = "weather")]
#[derive(Serialize, SchemaType)]
enum Weather {
    Rainy,
    Sunny,
}

#[init(contract = "weather", parameter = "Weather")]
fn weather_init(ctx: &impl HasInitContext) -> InitResult<Weather> {
    // let initial_weather = match ctx.parameter_cursor().get() {
    //     Ok(weather) => weather,
    //     Err(_) => return Err(Reject::default()),
    // };
    let initial_weather = ctx.parameter_cursor().get()?;
    Ok(initial_weather)
}

#[receive(contract = "weather", name = "set", parameter = "Weather")]
fn weather_set<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut Weather,
) -> ReceiveResult<A> {
    assert_eq!(Address::Account(ctx.owner()), ctx.sender());
    *state = ctx.parameter_cursor().get()?;
    Ok(A::accept())
}

// To the slides!

#[receive(contract = "weather", name = "get", parameter = "OwnedReceiveName")]
fn weather_get<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut Weather,
) -> ReceiveResult<A> {
    match ctx.sender() {
        Address::Account(_) => Err(Reject::default()), // Only invokeable by contracts.
        Address::Contract(contract_address) => {
            let receive_name: OwnedReceiveName = ctx.parameter_cursor().get()?; // Name of callback function.
            Ok(send(
                &contract_address,
                receive_name.as_ref(),
                Amount::zero(),
                state,
            ))
        }
    }
}

// To the slides!

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[contract_state(contract = "icecream")]
#[derive(Serialize, SchemaType)]
struct State {
    weather_service: ContractAddress,
    current_state: StateMachine,
}

#[derive(Serialize, SchemaType)]
enum StateMachine {
    ReadyToBuy,
    WaitingForWeather { icecream_vendor: AccountAddress },
}

#[init(contract = "icecream", parameter = "ContractAddress")]
fn contract_init(ctx: &impl HasInitContext) -> InitResult<State> {
    let weather_service = ctx.parameter_cursor().get()?;
    let current_state = StateMachine::ReadyToBuy;
    Ok(State {
        weather_service,
        current_state,
    })
}

#[receive(
    contract = "icecream",
    name = "buy_icecream",
    parameter = "AccountAddress",
    payable
)]
fn contract_buy_icecream<A: HasActions>(
    ctx: &impl HasReceiveContext,
    _amount: Amount, // Contract accepts the money.
    state: &mut State,
) -> ReceiveResult<A> {
    match state.current_state {
        StateMachine::ReadyToBuy => {
            let icecream_vendor = ctx.parameter_cursor().get()?;
            state.current_state = StateMachine::WaitingForWeather { icecream_vendor };
            Ok(send(
                &state.weather_service,
                ReceiveName::new_unchecked("weather.get"),
                Amount::zero(),
                &ReceiveName::new_unchecked("icecream.receive_weather"), // The callback function
            ))
        }
        StateMachine::WaitingForWeather { icecream_vendor: _ } => Err(Reject::default()), // buy_icecream should only be called when the contract is ready to buy.
    }
}

#[receive(contract = "icecream", name = "receive_weather", parameter = "Weather")]
fn contract_receive_weather<A: HasActions>(
    ctx: &impl HasReceiveContext,
    state: &mut State,
) -> ReceiveResult<A> {
    let action = match state.current_state {
        /* receive_weather should only be called when
         * contract is waiting for weather*/
        StateMachine::ReadyToBuy => return Err(Reject::default()),
        StateMachine::WaitingForWeather { icecream_vendor } => {
            match ctx.parameter_cursor().get()? {
                Weather::Rainy => A::simple_transfer(&ctx.invoker(), ctx.self_balance()), /* Return money to invoker. Not the right weather for icecream. */
                Weather::Sunny => A::simple_transfer(&icecream_vendor, ctx.self_balance()), /* Buy the icecream!! */
            }
        }
    };
    // Reset the statemachine.
    state.current_state = StateMachine::ReadyToBuy;
    Ok(action)
}

//////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////////

#[concordium_cfg_test]
mod tests {
    use super::*;
    use test_infrastructure::*;

    const ICECREAM_ADDR: ContractAddress = ContractAddress {
        index: 0,
        subindex: 1,
    };

    #[concordium_test]
    fn test_weather_service() {
        // Arrange
        let mut ctx = ReceiveContextTest::empty();
        let receive_name = ReceiveName::new_unchecked("icecream.receive_weather");
        let parameter = to_bytes(&receive_name);
        ctx.set_parameter(&parameter); // The callback function.
        ctx.set_sender(Address::Contract(ICECREAM_ADDR));

        // Act
        let weather_action: ActionsTree =
            weather_get(&ctx, &mut Weather::Sunny).expect_report("Calling get failed.");

        // Assert
        assert_eq!(
            weather_action,
            ActionsTree::Send {
                to: ICECREAM_ADDR,
                receive_name: receive_name.to_owned(),
                amount: Amount::zero(),
                parameter: to_bytes(&Weather::Sunny)
            }
        )
    }
}
