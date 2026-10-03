# Testnet grant run

Run 2026-10-03T09:54:26.273Z against `CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP` with the BGT test asset `CBEOUEMDTM56NRTOOSXCPGMD33RPD5YWEKJ542HAEVREAYCC4QIZ3XIU`.
317 of 317 checks passed across 154 transactions.
Highest fee paid by one transaction: 9960118 stroops.

## setup

- pass: BGT CBEOUEMDTM56NRTOOSXCPGMD33RPD5YWEKJ542HAEVREAYCC4QIZ3XIU is whitelisted on CBEODVJGUYCIYTVXD7KI5UG3BJ2UE4T7AGI2TGY3T4Q5GQRFGTRYVTZP (2.0.0)

  - [provision 2 accounts](https://stellar.expert/explorer/testnet/tx/43303068e112738cf13833744066d00d229a70e5c25c51f502226416039ada3a)
  - [provision 4 accounts](https://stellar.expert/explorer/testnet/tx/002146647b62635b45fb6f35cdee157d54c93fd8abbe4a4539b5e51c07f720f7)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/554d2ba1cefa32940a2d7151a747a585298f750f20f51ddfb37fb9f6e553273b)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/5c0f71439b56b697d412fc30dac58486cec7045a652864e6ced75e152153a6f9)
  - [provision 15 accounts](https://stellar.expert/explorer/testnet/tx/5300389140df301654a3d58a810899819c71288f8c465b915eecf132689ebe0a)
  - [provision 7 accounts](https://stellar.expert/explorer/testnet/tx/742b0face71117cc9d21727840dcfaf37352cc7f038f606aa64d0760b788713d)
  - [provision 3 accounts](https://stellar.expert/explorer/testnet/tx/daa6c6ba64cf718074bdce4e62480376b94fc63b48667de2503135cf6d60b23b)

## micro grant

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select three recipients
- pass: release milestone 0 to GBLCHA
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAIEF4
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GANEUH
- pass: milestone 0 paid exactly
- pass: completes when the last award is paid

  - [create_event](https://stellar.expert/explorer/testnet/tx/c78aaee5c34906b7fa8ed87b43c9af097423dd4653db3929dd86c5fb45a15757)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/9d7c855a6d47a18d332a326b8422d45bdf2de7f1dab54e225ae6c7fc9b7117ad)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b00670f6823f92661ffbd429ba12fcd14ef59065edc60c3afd7cce59d8240ce0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5297057a05fca836240612f38e82c6ec32ce40447fcb0666ab80d28de4770845)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9c1754c5c4fee7a0b6fb776d2ad393f30c0e0ca771d5ffc5f45fe3a63b07c795)

## tiered, rounding, any order

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select three tiers
- pass: release milestone 2 to GALEW6
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GB6AIZ
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GCYQYI
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GB6AIZ
- pass: milestone 2 paid exactly
- pass: release milestone 0 to GALEW6
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCYQYI
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GB6AIZ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GALEW6
- pass: milestone 1 paid exactly
- pass: release milestone 2 to GCYQYI
- pass: milestone 2 paid exactly
- pass: tier 1 received its whole award
- pass: tier 2 received its whole award
- pass: completed

  - [create_event](https://stellar.expert/explorer/testnet/tx/b00550c3c7ca97ee60501bfd868513fb38c97699f9669abce3dd1d164e9e693d)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/d881b76e9dabefe4ea05f37ab701935be769165efda12a2f3eb3a981c5f50108)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e109535e089af6a1c9cf81792554aab640bf4c2843bcf0357dc1c1596f2c3ba2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b756afc0925d0d67f9fefefa8b9284c06d30c2eff13c9f14fc3bf09b764e916e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0f62f243970658b4da8b42a06d1e82b180a5d031d4b1ec1168ab6606d741e415)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5cbb43cf4f5f72243323197a7c2ef88adaa0a142a02bb38db646d5769a075dc7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d135646c70434dee93b2e08f19ce6ac7cf9021f4ac8150d576654e30d2602990)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c649fe3cfb2d5df62cf56f2399fb0948bfd4fc67f951b22ef6ce104a85849f1b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c0a600718528b2464814d3dd57d3479336aeb0f43e48f513648cd5b8b4e79d7c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/754a43cfd99536d09432f7993dd8a871273eeee07372f27306047a3c9959ddff)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f37c7bee0109eabd5c3c7f834af3ac694677e2ebbf99a3096efa32d6d3b4b67b)

## forfeit and the cancel guard

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select one recipient
- pass: release milestone 0 to GCJZKP
- pass: milestone 0 paid exactly
- pass: forfeit milestone 1
- pass: a forfeit pays nothing
- pass: cancel refused while an award is owed (AwardsOutstanding)
- pass: release milestone 2 to GCJZKP
- pass: milestone 2 paid exactly
- pass: start cancel
- pass: grant cancelled
- pass: forfeited share returned to the owner

  - [create_event](https://stellar.expert/explorer/testnet/tx/77003b2f998fa3cf955e76d01700569077bdaabe5c8257a8dbfc4cc8c83ab876)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/6af3ca56e39cb1a97040437e61ccc363c9462c7ac7ae51f6acfc616295afbfac)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5eccc98f1df853ad1f2e1b56933e8547fc4cf6f030d650e2fa0c9585c23a5e52)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/2beb87c625d7fa2de5fe4a417a759fdadf7e63959962175dea54e64e260cc192)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/fde369b7583129da470cc337e8ff7524fb9bf2dffde6718bc73c5bf2203aef8b)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/c61138719114b9641ed630e8ec0c73c016a4341a836807e9ce0afc9af58c41ca)

## partners, pro rata

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: partner adds 20.0000000
- pass: partner paid amount plus fee
- pass: award more than the budget, funded by the partner
- pass: release milestone 0 to GDZSRB
- pass: milestone 0 paid exactly
- pass: partner adds 15.0000000
- pass: partner paid amount plus fee
- pass: release milestone 1 to GDZSRB
- pass: milestone 1 paid exactly
- pass: the late top-up keeps the grant open
- pass: start cancel
- pass: an empty refund batch is refused (InvalidBatchSize)
- pass: anyone can crank
- pass: anyone can finalize
- pass: partner 1 refunded pro rata
- pass: partner 2 refunded pro rata
- pass: owner gets nothing when partners are short

  - [create_event](https://stellar.expert/explorer/testnet/tx/d3d47c20da243c51c3b3278024b82bd0d4444085e61ac2d3dc038ab74b942989)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/cc75425875de191ddd8580e1edced083a529954f12b855f68d8e253334b7cdf3)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/c02a6528a76c8168a4ead1fa0d4ba87144e1ad7fd170e4b7c745986b31802648)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c55d83171df79dcc30f0ca4eb225656193bf5fac7fc0254c20bfc923ff214c5c)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/7b85bffe6de0c88e0df70e97787e3486bb7ad4adfa9dfaf6dd7a015c9067703e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/439bf8db026b93b4a1b821985a3899146f3c5ef956c2e60ae6e1999db377b084)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/a81c1a0ad8cb87c17e0d5cbae268d692ad0d699cce4a8de3a45a5b0bcc2eb290)
  - [process_cancel_batch](https://stellar.expert/explorer/testnet/tx/d8db627f692452c126ba4b0add8eb7f1c2154949991c679da59e00d93ab0f55a)
  - [finalize_cancel](https://stellar.expert/explorer/testnet/tx/44416739a3d8f722722586abd78722174acbc00c823700f5e86f2de2a9d2a2ca)

## frozen partner

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: partner adds 10.0000000
- pass: partner paid amount plus fee
- pass: partner adds 10.0000000
- pass: partner paid amount plus fee
- pass: start cancel
- pass: refund batch
- pass: finalize cancel
- pass: grant cancelled
- pass: other partners are refunded
- pass: owner residual paid
- pass: frozen partner refund held
- pass: a frozen account cannot take its refund yet (HostError: Error(Contract, #11))
- pass: refund claimed after unfreezing
- pass: frozen partner made whole
- pass: a second claim is refused (NoRefundOwed)

  - [create_event](https://stellar.expert/explorer/testnet/tx/16a9363b0b6c8c783e6d7a185fed67965505b943d354d2feed492c20ec218004)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/ce27b401a530afba374f8852a76ada34146239181c7be39fbe5225c6177461c9)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/41cb91b5fc940f953e813ba422fd697a90418b82dcd35d5bfd60f4296f808eaa)
  - [freeze](https://stellar.expert/explorer/testnet/tx/bc6b9e9167e162ab615f64eb6ed7989c986d830b27780306365b8c8880a5473d)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/64f07cb54e3516be8628bd0019702c4f47389d844df751cabd09181b6a6f0a50)
  - [process_cancel_batch](https://stellar.expert/explorer/testnet/tx/83059189b58b0fc9d60ab710d549967b496e015ceead6ab2f3d8a30ba29c6ddc)
  - [finalize_cancel](https://stellar.expert/explorer/testnet/tx/abc9c8514cda57bf11bb1789dba56c60d8cd3eff948024512cdd722a597dbf5a)
  - [unfreeze](https://stellar.expert/explorer/testnet/tx/8d3bf81c3c92e6e4432cfa19d458021241cd0060cf0a690c0c00908b2a865ffa)
  - [claim_refund](https://stellar.expert/explorer/testnet/tx/2a5beecc43a8abaec0fa01ab31e49d0dea6b70cda95ccbc377b286aa341891df)

## refusals

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: one recipient, two awards (DuplicateRecipient)
- pass: award smaller than its milestones
- pass: reputation bump over the cap
- pass: more than 40 recipients
- pass: awards beyond the escrow
- pass: select
- pass: a second selection
- pass: milestone out of range
- pass: someone never selected
- pass: release milestone 0 to GDVTCZ
- pass: milestone 0 paid exactly
- pass: the same milestone twice
- pass: release with an op id
- pass: the op id replayed
- pass: reputation bump over the cap on release
- pass: claim_prize is not a grant path
- pass: cancel refused while owed
- pass: forfeit milestone 2
- pass: a forfeit pays nothing
- pass: forfeit milestone 3
- pass: a forfeit pays nothing
- pass: start cancel
- pass: grant cancelled

  - [create_event](https://stellar.expert/explorer/testnet/tx/7e028af44aae9cd078f926107acdc1b33c27cefda7a74275ae9b919d055ef886)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/342b6b2f998ff324e296d72f00afa05e667646c081da8162b41df687a8157879)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ca8c1cc89ab2b408e37639c6ce0c396c9e51bb10963a18afb7c7d59dc0135df4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2741ae25686d1e147a9775ddac68139d86c0c516815aace0485d9c5138326b5b)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/373f3be1be07819fb54874a3d5a4c5d49e9833260b4314eb10def97ee278615b)
  - [forfeit_milestone](https://stellar.expert/explorer/testnet/tx/0c4997179369b379ad780f637e2eddab38a69f004ac619f6947e89033abc59d3)
  - [start_cancel](https://stellar.expert/explorer/testnet/tx/c09ee7faac9cef813205ee2feb4e750eb1efb319d93587fd1dc6c4b3401fc5bc)

## signatures

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: a stranger cannot select (HostError: Error(Auth, InvalidAction))
- pass: owner selects
- pass: a recipient cannot release their own milestone (HostError: Error(Auth, InvalidAction))
- pass: nothing moved
- pass: release milestone 0 to GDRPDG
- pass: milestone 0 paid exactly

  - [create_event](https://stellar.expert/explorer/testnet/tx/2b71a63beb441c4b34246d0eb965bea12a46e4a5b801fb7f44a5e32389d46944)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/05feda61bb92c1421e4346cbb5cafdc26709ea0f26a56a591b26177cc4abc290)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/a31ff6bb819736552d8ab7efa435ca415782760281b3d9eb7eab8c13eb467d1d)

## manager handover

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: manager accepts
- pass: manager recorded
- pass: the owner no longer selects (HostError: Error(Auth, InvalidAction))
- pass: the manager selects
- pass: no reclaim after selection
- pass: the manager cannot release (HostError: Error(Auth, InvalidAction))
- pass: release milestone 0 to GBK2R7
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GBK2R7
- pass: milestone 1 paid exactly
- pass: publish the grant
- pass: owner paid budget plus fee
- pass: second manager accepts
- pass: owner takes management back
- pass: the owner manages again
- pass: owner selects after reclaiming
- pass: release milestone 0 to GBK2R7
- pass: milestone 0 paid exactly

  - [create_event](https://stellar.expert/explorer/testnet/tx/9502d33390c6cbeda86184dbceb57850720f9ecdd411d799403acd00aa7fda55)
  - [accept_manager](https://stellar.expert/explorer/testnet/tx/e5a5bd071a6f181d78402a3e3548ec112ef4d9af44cc36da9ebd3c21ac099db0)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/a93f3c8a13e5431b26fa0a03cb8d4024513b476084fe8e871ea599f3509d01d6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3bb77fa810e1b64e6acfdd72ef6b517b7389aeab6ae0f96eadb90f8cb916d72b)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/88449cc8546fa17cb4fe00b5030f31c31f6319f838dd308743013a1fdcc02b78)
  - [create_event](https://stellar.expert/explorer/testnet/tx/6888610da9fd60ebfb59601fd258742058420f677b86a1a85d41c620966910a8)
  - [accept_manager](https://stellar.expert/explorer/testnet/tx/d34fbe4bec22e23c2c0394cce001ece3bf88f81ea4d11ef964e5274d40e0fcfe)
  - [reclaim_management](https://stellar.expert/explorer/testnet/tx/18d6ad962b6bac60c23b40b84e4f283b9a17c5d1dc1d6c9f901e4dff9e335644)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/4e8cb52a087f28a6cb895087df441a5c9d197821bb26b46f9f3ee776666eb871)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/0034e09ad229e94d1d20be7508f8d7f1ca3656f125a6412cb6ab2a9a6f934527)

## 40 recipients

- pass: publish the grant
- pass: owner paid budget plus fee
- pass: select 40 recipients in one transaction
- pass: release milestone 0 to GAKMUH
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBD6NQ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD7YBV
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCCSIY
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCCSTJ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDAOQ7
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAB5YH
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBO2RD
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBEDOP
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAFVE4
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDI35S
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCNFJU
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBEFLB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAYNV3
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCDMZN
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDQXCV
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBLZK6
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBW24G
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDBJBB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GC6ZPO
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCIHKL
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCVCHE
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBBVFG
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBYLFZ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD7QSB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GALB4H
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD3M5F
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBB7PT
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GD76RN
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBAX6W
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAMOO2
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBICRD
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDTZXJ
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBNKH7
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBTQI5
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GAOFJB
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GBHTHY
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GABOR6
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GCRIDA
- pass: milestone 0 paid exactly
- pass: release milestone 0 to GDONJ3
- pass: milestone 0 paid exactly
- pass: release milestone 1 to GAKMUH
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBD6NQ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD7YBV
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCCSIY
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCCSTJ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDAOQ7
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAB5YH
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBO2RD
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBEDOP
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAFVE4
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDI35S
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCNFJU
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBEFLB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAYNV3
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCDMZN
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDQXCV
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBLZK6
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBW24G
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDBJBB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GC6ZPO
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCIHKL
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCVCHE
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBBVFG
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBYLFZ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD7QSB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GALB4H
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD3M5F
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBB7PT
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GD76RN
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBAX6W
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAMOO2
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBICRD
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDTZXJ
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBNKH7
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBTQI5
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GAOFJB
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GBHTHY
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GABOR6
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GCRIDA
- pass: milestone 1 paid exactly
- pass: release milestone 1 to GDONJ3
- pass: milestone 1 paid exactly
- pass: completed
- pass: a full page of winners reads in one call
- pass: the rest on the next page

  - [create_event](https://stellar.expert/explorer/testnet/tx/39f055782bff602de073ad7012f7386b809edfeebcdaf2d77ce58118498dbbc3)
  - [select_winners](https://stellar.expert/explorer/testnet/tx/d0bedebfec73df2726d52d80d57b8f20cb9e22dd633d102c85f22b50ee2a1248)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7d24da85a0952d04c4e7e5736614e867d63360100aa019af67ffd8ee84a64656)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/624638c8eaba72fe939387e9f8c501147c0a352b1ef116c2ef716c5fd7856081)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2855ceb466fd0297fcdd55752b32a053f7ea7ecc948fbe2a122c168f017b3753)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4193002a990e474aeace68a1670201a986bc37d897f46f829390da1a2381cc26)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1ece78e6d0dd990e11e3ad7aa9a48e17ca52c029360b98aebf80c306a6fc4bb3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/cb090dfda26bc52a2efda3a7a5197a731fea5f7b056f68088fb1bcae38177141)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e6ab0fd69d13513a437499bcc951475200c07043961658a6fe9495cc9e5d8f24)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c127f9d957faf3fbbca155d00b6a9c0175451d1f815979eabdc4e87b54f12bf2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b40ed526d727b7ddcbff9c0909bedb2413fbe967d7177036516c512674a9a264)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7ec5eb862ed0b5574a738ffe185e27db381e55c37c6165ac93686aff8e257b5c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d7a8821d2a826c09aafa89189d7e55f86e205632fc981ffb1c03a5e58264b4b3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/13db432c3f74a8e4c18d708d30be33695569ca14f0f5b02f9f708b34d9201c41)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/73cc1f3ee092bc49706cff503212e6849b61941fcd8174a2aeba7faf30dcdb62)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/263d04242bd7eab5a46129b7aa66725b870e46f4e1cab0dfcd5853fa537c27d0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/450084e817e393a57de11e8e82c60384fb690545a3729c940bd3c56a3669399d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c9c12539914a5f10466848441fcaca7759cf668cd8a88e773c7b0e6889a1c0fa)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8a257ed77e58e07e8155c4e71c8bf3bce5fde09c77ff285e87e192b3ca05334f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/37a561a4e9bbcdbab194ae8c0eb239217d779824410117ec47db27592ccb5c19)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/b89f04a5010a11d332c2f1bf95aa41a960444987c98b76d36dd4a9f69c26e5d4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/79382996bfd877f8d60e7eac75be0bf134954513d9d48505a2fefcc1cda251f4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bc101f28428850835d57fb468c4d96fa612d0d2465817a0ad9455394cc2ae9f8)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/ee0a0320e15f2a878517df445df9bdfb02b5692190a8ee314bca6315bb1b608d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2618e9c0ac839da5b4a05b08e79d0a68a831f968302a2ac08dd9273b4550e541)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/37207648a3ccedf6f01283c140c44ce038bd3e9e7a59d39cd72e41c48dd74d59)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/79662d23cd467760236b213f0c266ca0b8f4e17e29d0cd555e558fcd5bd53f2f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/916eaf4628b701dc1f002ad5f35cd30480eb57f97b3a056a8cacf05e6de183a4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/fe2240ea2333acf3728ccde8df69546d28915e7108cc0d495a80d103372af823)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9e5b85468e8a3ebcd9a895e8c50f3de1d775ddfe08a7acb15e542dc9aced94d7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6648715962606750261969920b3ca265d90f965d3b60799bb8c0ca3d75ad37d0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2650c15c684b6f5a66d5d649cb631757d74c6d8436a7c2b9067c1a450a3d3d1c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3b52bb83cf88a0c7940ee03694d6e5c311ed19e151c15bba5559370d8095c265)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2580089a31de777b0ff8a56b68f9a2c1a03be2c86e512a8a03834badfb85b2f0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/304efea234093a453a5d8bff24c50f108e2812afe8bb980f57df5d5adde31347)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/888109b134317ea37d43288100a0d4f8f9c57b448044ddaf84c0c3516f0795a9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/f5a2f96d1160648e89bcd36d246c15d166ff14fec431e02fac2b6a993a6f7ce7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/75b289cd61d96c19c3fb26dd2f139b65143da87b6088b7895500f867fe886ac9)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/497f4f47d7ea7e4233d548a0c827286dfdcc3bc20a78cf54b154892df36dd6e3)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e93292b30a53db1205c923236fb5d62bc17a0b807e2aef39ae6a2b46a44f5a63)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/69895c4b96f8eaf2708461a21c89a905d5a5306ad7b42a089eecd079bb6574cc)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/dd0139a91682a729731f6d6aa101268016658ad4a7edd6bf7923a7c2a176ae19)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/08fa620e5c54c14cc651374f3f74348038fa2d2ae2b5478405851e75be509eb1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d478fbee228e9dde9c59d9c0712fba8d5627db7ce7f5bdcdc9206b9736d48ffa)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/928a70371745ee82c1016c971736c8419b13bc00581be363d3aa452b649c6859)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2f1827409f29bef2416017d6fa3e9839252858558dc72f65e740be388a43a2e6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/069ecab5d29a6f49b62f1f4b8122637f3d42ea72d189a8911f3c54fae3ebfd45)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/bc1ad2494ee930516c10acf9d88396930307cb17c39559106cafb817a6080725)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/5fde924f77309d39f9cb5759b4e544e91a15d401267fc8fe012d1fba5548de73)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/07679b46c1dbb795361d67287f710435e7cc9ef4818f70daa67909a7f52d735d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9b94c712fdb2c7cde69dfc53a0c7b422c9f87529991b86ff7cf4279bdb15c833)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/84064bd42348fc95f845f1ac9c9a16976ddacfab468a90a762986e1ca1b6cc84)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/3021f51f15d092b44b2e654ca8236cf5ccbbd8b851a715255688045ce342a93e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/cc71ac298197e1c3872a151523d74ff6bb5ef7254cfc7e7421d2698c52b937ee)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/48c6f5243503096dbfdde142db0f939901bd52b803cf6c3ce5d11f94596fece6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/fcf2d01a896eca17bd3d1b068f20f86b6fb465832f57477e63226716c6456b12)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2aa6590ea339cf29d32ef9af7cef8e9437892276604f0d21aed3bbc403dfd03e)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6a1332a7d84a24f4462e8411c71cb5b7c8d50b398912cb9edd3e355574624dcb)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c2dd0e228dfbcc22056c325e0a60408ab5d5d67996474108f4c49a3f6495f9d7)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/1e89f9838ae13b6d6e692aab11196e9068b526577a17aaa159e4ea0307e244b2)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2aec80d040106fa6b7e8583063d00b55e6ee9656966675c7ce3130ac3ed7527a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/56db084e505fa38834f3644231841b69942f091245df52ac636fbcccc6eaa223)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/78a0716bb2c49a388ccfe58d5dad53afcb4f7e45d9cf2ce5e72f1db44485c7eb)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/817cc5d37c79eefd5194de9d7a397782fca7fb87ba166fe8b04af538db6d69fa)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/88ef8bbcb2b6751b664fabd17b42f53c6d309ed32040a063027e16670064e845)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/c9964c27ae249cc37084e961ae6fa9689bc42a59c4690dc1c56618d2e8a68150)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7e7caeac35ef57067cfd2d61c1bbe22f930f74bf385587dc5945caa21d81efb4)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2233e2f44be51cecd41a24dd8452f1b821b777b58b2c6cfca38859c25c71200f)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2547359703b37e6731c30a8bf7c59ad81e3f6f422aca7c2d50bdf3dbf0d0dfb1)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/87cf4f5a3fb4c7ad7916d1e18538b810911dafe61d5abc9de982f4eda692916d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/4e3480fa348a61355b5ab0c80fc390bdd07566a377e105320f8d7ef9a0be2209)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d4c11647e8d08ade736d3b34818898bd7d78e4f01d901bfe6e5e5d617394bb52)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/7b75718e76591f37de3b952e1495109c91ebd9b107d5b1fc3f23d43bc436350d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/e8b569a895e9abc23e9104869758aa3700fcc6d955eb1a519a8a723861600fa0)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/2c7986cb7b190d29384084a2f0805288b0e2c402e37ed761524472025b024ce5)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/d4313699d0144672ef40b1b196da9c1b7843669e8578b96b72e6086d123fbeeb)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/428f3ea10e426c588587949f6f21950c027ec6029077db1aab261fdd6294fa1c)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/6e695162e4df6b031ebb7f7ab88ae6f28aaec4d34f3cdd3a70fa8e2c80e7343a)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/002af34286d49890e7aa10153dbb5872d560f7f814449208cf7cec4e55ec8601)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/19a65f9cc74e87236578da5e917fd1f077e80dfe7640b1ecbfef54186c21692d)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/fbefc5e44b6297a2614db67895b381f2643011ea9167b0e938ced308bf2d5999)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/8a5d9c64a40aaf548ab8cb19c8c4c1262303a5b7c817b4cc8e1756ba7e106839)

## crowdfunding release validator

- pass: the validator is appointed on chain
- pass: publish the grant
- pass: owner paid budget plus fee
- pass: a backer pledges 60
- pass: backers pay no fee
- pass: the creator alone cannot release (HostError: Error(Auth, InvalidAction))
- pass: release 0 co-signed by the validator
- pass: creator paid net of fee
- pass: fee withheld at release
- pass: release 1 co-signed by the validator
- pass: creator paid net of fee
- pass: fee withheld at release
- pass: campaign completes on its last release

  - [provision 1 accounts](https://stellar.expert/explorer/testnet/tx/aa0dcce2f72b8e642a0cf76d83da9fd933d9ce7ce9acc74f53fc151f0d983942)
  - [create_event](https://stellar.expert/explorer/testnet/tx/68adb492b228b0b9590d6ad163e1b0e43b3a1325905c7a33130df1752f0f7024)
  - [add_funds](https://stellar.expert/explorer/testnet/tx/13216bdefec6754b37ffebe5042baca9406c0a1debdc3345cd71539f069f74d6)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/9c59bbf88a6282be59b193dd0c11c896847c14229ddcd7ef14f88bb96cd44251)
  - [claim_milestone](https://stellar.expert/explorer/testnet/tx/97ed9a410eac75b13ca2f565eaa96c07eff9af43fbf52901f9993aaa14bf5dde)

## fees

- pass: fee account received exactly the fees charged

