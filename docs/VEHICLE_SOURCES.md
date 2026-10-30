# Références mécaniques et combiné — Mégane II Sedan 1.6 16V (2004)

## Calibrage **provisoire** du simulateur

- Motorisation Renault **K4M** 1 598 cm³ ; 83 kW (113 ch) à 6 000 tr/min ; 152 Nm à 4 200 tr/min.
- Transmission mécanique : **JH3 142 à confirmer sur la boîte installée** (type de boîte/indice sur plaquette). La documentation MR364 associe une JH3 142 au K4M 760 B/C/M0C. Le code carte grise **LM0J0H** et le moteur exact ne démontrent pas à eux seuls que la boîte de ce véhicule précis est JH3 142.
- Hypothèses JH3 142 d'après tableau de rapports documenté : 1re 3,72, 2e 2,04, 3e 1,39, 4e 1,09, 5e 0,89, marche arrière 3,54, couple final 4,06. **À vérifier** pour ce véhicule.
- Pneus par défaut : 195/65 R15 (diamètre théorique), variantes 205/55 R16 et 205/50 R17 selon monte réelle.
- Masse roulante du modèle : 1 350 kg **hypothétique** avec conducteur. Coefficient aérodynamique × surface 0,70 m² **estimé**, rendement transmission 88 % **estimé**, coefficient de roulement 0,012 **estimé**.
- Couple moteur : seuls les points 4 200 tr/min (152 Nm) et 6 000 tr/min (83 kW ≈ 132 Nm) sont ancrés dans des données constructeur. Tous les autres points de la courbe, l'embrayage simulé, la traînée, la résistance au roulement, les limites et les contrôleurs de vitesse sont **simplifiés**. Il s'agit d'un comportement cohérent en ordres de grandeur, **pas d'une reproduction certifiée Renault**.

### Sources consultées

- RTA MR364 Renault Mégane II, correspondance K4M 760 ↔ JH3 142 : https://www.technirevue.com/revue_technique/revue-technique-renault-megane-2.pdf
- Table de démultiplication JH3, JR5, ND0 (types et indices) : https://pisarenko.by/en/hobbies/car/megane2_gearbox.html
- Pneus d'une Mégane II 1.6 16V : https://www.allopneus.com/vehicule/renault/megane/megane-ii
- Caractéristiques transmission et moteur : https://www.autotitre.com/fiche-technique/Renault/M%C3%A9gane/II/1.6_16v/2002-2005
- Documentation technique combiné 83A, ouvrants et TPMS : https://www.manualzz.com/doc/o/y96uw/renault-megane-ii-manuel-utilisateur-83a-instrument-tableau-de-bord
- Historique TPMS Mégane II : https://www.renaultbook.ru/en/Megane/2_p/main/drive/sistema-kontrolya-davleniya-vozduha-v-shinah
- Pressions 1.6 16V, manuel historique (selon pneumatiques/conditions) : https://www.man4car.com/5/1451776810/Renault-Megane-II-2-owners-manual

## Ouvrants et pression des pneus

Le simulateur permet d'ouvrir séparément les quatre **portes**, le **coffre distinct du hayon** et le capot. Les quatre pressions de pneus sont indépendantes et l'on peut simuler un capteur manquant.

Pour les pneus, les pressions initiales 2,2 bar avant et 2,0 bar arrière sont **des valeurs de scénario** fondées sur un tableau de manuel historique, **pas une recommandation pour la voiture** : se référer à l'autocollant du montant de porte et à la monte réellement installée. Les seuils virtuels d'alerte par roue (réduction de 0,30 ou 0,65 bar) sont inventés pour tester l'interface, pas les seuils réels du calculateur.

**Aucun de ces signaux n'est garanti par OBD-II générique.** Une future source OBD sera strictement en lecture et fournira `Unknown` quand la donnée n'est pas disponible ; aucune information ne sera inférée à partir du régime, de la vitesse ou du bus CAN sans décodage validé. Le système réel d'origine doit rester installé et fonctionnel.
